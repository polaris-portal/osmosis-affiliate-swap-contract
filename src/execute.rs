use std::str::FromStr;

use cosmwasm_std::{
    coins, Addr, BankMsg, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128,
};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSplitRouteSwapExactAmountIn, MsgSplitRouteSwapExactAmountInResponse, MsgSwapExactAmountIn,
    MsgSwapExactAmountInResponse,
};

use crate::error::ContractError;
use crate::msg::{ProxySwap, SwapResponse};
use crate::state::{Config, PendingSwapKind, SwapReplyState, CONFIG, SWAP_REPLY_STATE};

pub const SWAP_REPLY_ID: u64 = 1u64;

fn assert_owner(deps: &DepsMut, sender: &Addr) -> Result<(), ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if cfg.owner != *sender {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn update_affiliate(
    deps: DepsMut,
    info: MessageInfo,
    affiliate_addr: String,
    affiliate_bps: u16,
) -> Result<Response, ContractError> {
    assert_owner(&deps, &info.sender)?;
    if affiliate_bps > 10_000 {
        return Err(ContractError::InvalidAffiliateBps {});
    }
    let addr = deps.api.addr_validate(&affiliate_addr)?;
    CONFIG.update(deps.storage, |mut cfg| -> Result<Config, ContractError> {
        cfg.affiliate_addr = addr;
        cfg.affiliate_bps = affiliate_bps;
        Ok(cfg)
    })?;
    Ok(Response::new().add_attribute("action", "update_affiliate"))
}

pub fn transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    assert_owner(&deps, &info.sender)?;
    let new_owner = deps.api.addr_validate(&new_owner)?;
    CONFIG.update(deps.storage, |mut cfg| -> Result<Config, ContractError> {
        cfg.owner = new_owner;
        Ok(cfg)
    })?;
    Ok(Response::new().add_attribute("action", "transfer_ownership"))
}

// Single proxy endpoint
pub fn proxy_swap_with_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap: ProxySwap,
) -> Result<Response, ContractError> {
    match swap {
        ProxySwap::SwapExactAmountIn {
            routes,
            token_in,
            token_out_min_amount,
        } => {
            // validate funds exactly equals token_in
            if !info
                .funds
                .iter()
                .any(|c| c.denom == token_in.denom && c.amount == token_in.amount)
            {
                return Err(ContractError::InsufficientFunds {});
            }
            let msg = MsgSwapExactAmountIn {
                sender: env.contract.address.into_string(),
                routes: routes.clone(),
                token_in: Some(token_in.clone().into()),
                token_out_min_amount: token_out_min_amount.to_string(),
            };
            let out_denom = routes
                .last()
                .map(|r| r.token_out_denom.clone())
                .unwrap_or_default();

            SWAP_REPLY_STATE.save(
                deps.storage,
                &SwapReplyState {
                    original_sender: info.sender,
                    token_out_denom: out_denom,
                    kind: PendingSwapKind::Single,
                },
            )?;

            Ok(Response::new()
                .add_attribute("action", "proxy_swap_with_fee")
                .add_submessage(SubMsg::reply_on_success(msg, SWAP_REPLY_ID)))
        }
        ProxySwap::SplitRouteSwapExactAmountIn {
            routes,
            token_in_denom,
            token_out_min_amount,
        } => {
            // sum input
            let mut total_in = Uint128::zero();
            for r in &routes {
                let amt = Uint128::from_str(&r.token_in_amount)?;
                total_in = total_in
                    .checked_add(amt)
                    .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
            }
            if !info
                .funds
                .iter()
                .any(|c| c.denom == token_in_denom && c.amount == total_in)
            {
                return Err(ContractError::InsufficientFunds {});
            }

            let msg = MsgSplitRouteSwapExactAmountIn {
                sender: env.contract.address.into_string(),
                routes: routes.clone(),
                token_in_denom: token_in_denom,
                token_out_min_amount: token_out_min_amount.to_string(),
            };
            let out_denom = routes
                .first()
                .and_then(|r| r.pools.last())
                .map(|p| p.token_out_denom.clone())
                .unwrap_or_default();

            SWAP_REPLY_STATE.save(
                deps.storage,
                &SwapReplyState {
                    original_sender: info.sender,
                    token_out_denom: out_denom,
                    kind: PendingSwapKind::Split,
                },
            )?;

            Ok(Response::new()
                .add_attribute("action", "proxy_split_swap_with_fee")
                .add_submessage(SubMsg::reply_on_success(msg, SWAP_REPLY_ID)))
        }
    }
}

pub fn handle_swap_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let state = SWAP_REPLY_STATE.load(deps.storage)?;
    SWAP_REPLY_STATE.remove(deps.storage);

    let amount = if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result.clone()
    {
        match state.kind {
            PendingSwapKind::Single => {
                let res: MsgSwapExactAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                Uint128::from_str(&res.token_out_amount)?
            }
            PendingSwapKind::Split => {
                let res: MsgSplitRouteSwapExactAmountInResponse =
                    b.try_into().map_err(ContractError::Std)?;
                Uint128::from_str(&res.token_out_amount)?
            }
        }
    } else {
        return Err(ContractError::FailedSwap {
            reason: format!("{:?}", msg.result.unwrap_err()),
        });
    };

    let cfg = CONFIG.load(deps.storage)?;

    // Calculate affiliate fee and enforce a minimum fee of 1 unit when a non-zero fee
    // rounds down to zero. This ensures we always charge some affiliate fee on swaps
    // when affiliate_bps > 0 and there is a non-zero output amount.
    let mut affiliate_amount = amount.multiply_ratio(cfg.affiliate_bps as u128, 10_000u128);
    if cfg.affiliate_bps > 0 && affiliate_amount.is_zero() && !amount.is_zero() {
        affiliate_amount = Uint128::one();
    }
    let user_amount = amount.checked_sub(affiliate_amount).unwrap();

    let mut msgs: Vec<cosmwasm_std::CosmosMsg> = vec![];
    if !affiliate_amount.is_zero() {
        msgs.push(cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
            to_address: cfg.affiliate_addr.into_string(),
            amount: coins(affiliate_amount.u128(), state.token_out_denom.clone()),
        }));
    }
    if !user_amount.is_zero() {
        msgs.push(cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
            to_address: state.original_sender.to_string(),
            amount: coins(user_amount.u128(), state.token_out_denom.clone()),
        }));
    }

    let response = SwapResponse {
        original_sender: state.original_sender.into_string(),
        token_out_denom: state.token_out_denom,
        amount_sent_to_user: user_amount,
        amount_sent_to_affiliate: affiliate_amount,
    };

    Ok(Response::new()
        .add_messages(msgs)
        .set_data(cosmwasm_std::to_json_binary(&response)?)
        .add_attribute("token_out_amount", amount)
        .add_attribute("affiliate_bps", cfg.affiliate_bps.to_string()))
}
