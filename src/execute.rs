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
            // validate funds are at least token_in
            let gross_in: Uint128 = info
                .funds
                .iter()
                .filter(|c| c.denom == token_in.denom)
                .fold(Uint128::zero(), |acc, c| acc + c.amount);
            if gross_in < token_in.amount {
                return Err(ContractError::InsufficientFunds {});
            }

            // Compute affiliate as the difference between gross funds and provided net token_in
            let affiliate_in = gross_in.checked_sub(token_in.amount).unwrap();

            let mut resp = Response::new().add_attribute("action", "proxy_swap_with_fee");
            if !affiliate_in.is_zero() {
                let cfg = CONFIG.load(deps.storage)?;
                resp = resp.add_message(cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                    to_address: cfg.affiliate_addr.into_string(),
                    amount: coins(affiliate_in.u128(), token_in.denom.clone()),
                }));
            }

            // If nothing remains to swap, we are done
            if token_in.amount.is_zero() {
                return Ok(resp);
            }

            let msg = MsgSwapExactAmountIn {
                sender: env.contract.address.into_string(),
                routes: routes.clone(),
                token_in: Some(token_in.into()),
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

            Ok(resp.add_submessage(SubMsg::reply_on_success(msg, SWAP_REPLY_ID)))
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
            // validate funds are at least total_in
            let gross_in: Uint128 = info
                .funds
                .iter()
                .filter(|c| c.denom == token_in_denom)
                .fold(Uint128::zero(), |acc, c| acc + c.amount);
            if gross_in < total_in {
                return Err(ContractError::InsufficientFunds {});
            }

            // Compute affiliate as the difference between gross funds and provided net total_in
            let affiliate_in = gross_in.checked_sub(total_in).unwrap();

            let mut resp = Response::new().add_attribute("action", "proxy_split_swap_with_fee");
            if !affiliate_in.is_zero() {
                let cfg = CONFIG.load(deps.storage)?;
                resp = resp.add_message(cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                    to_address: cfg.affiliate_addr.into_string(),
                    amount: coins(affiliate_in.u128(), token_in_denom.clone()),
                }));
            }

            // If nothing remains to swap, we are done
            if total_in.is_zero() {
                return Ok(resp);
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

            Ok(resp.add_submessage(SubMsg::reply_on_success(msg, SWAP_REPLY_ID)))
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

    // Affiliate was taken from input; send entire output to user
    let mut msgs: Vec<cosmwasm_std::CosmosMsg> = vec![];
    if !amount.is_zero() {
        msgs.push(cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
            to_address: state.original_sender.to_string(),
            amount: coins(amount.u128(), state.token_out_denom.clone()),
        }));
    }

    let response = SwapResponse {
        original_sender: state.original_sender.into_string(),
        token_out_denom: state.token_out_denom,
        amount_sent_to_user: amount,
        amount_sent_to_affiliate: Uint128::zero(),
    };

    Ok(Response::new()
        .add_messages(msgs)
        .set_data(cosmwasm_std::to_json_binary(&response)?)
        .add_attribute("token_out_amount", amount))
}
