use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_json, Coin, Uint128};

use crate::contract::{execute, instantiate, query, reply};
use crate::execute::SWAP_REPLY_ID;
use crate::msg::{ExecuteMsg, InstantiateMsg, ProxySwap, QueryMsg};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSplitRouteSwapExactAmountInResponse, MsgSwapExactAmountInResponse, SwapAmountInRoute,
    SwapAmountInSplitRoute,
};

fn mock_instantiate<S, A, Q>(deps: &mut cosmwasm_std::OwnedDeps<S, A, Q>)
where
    S: cosmwasm_std::Storage,
    A: cosmwasm_std::Api,
    Q: cosmwasm_std::Querier,
{
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        affiliate_addr: "affiliate".to_string(),
        affiliate_bps: 250, // 2.5%
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
}

#[test]
fn test_config_query() {
    let mut deps = mock_dependencies();
    mock_instantiate(&mut deps);
    let bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let resp: crate::msg::ConfigResponse = from_json(&bin).unwrap();
    assert_eq!(resp.affiliate_bps, 250);
}

#[test]
fn test_proxy_single() {
    let mut deps = mock_dependencies();
    mock_instantiate(&mut deps);

    let routes = vec![SwapAmountInRoute {
        pool_id: 1,
        token_out_denom: "uosmo".to_string(),
    }];
    let msg = ExecuteMsg::ProxySwapWithFee {
        swap: ProxySwap::SwapExactAmountIn {
            routes,
            token_in: Coin::new(1000, "uion"),
            token_out_min_amount: Uint128::new(1),
        },
    };
    let info = mock_info("trader", &[Coin::new(1000, "uion")]);
    let resp = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(resp.messages.len(), 2);
    assert_eq!(resp.messages[1].id, SWAP_REPLY_ID);

    let resp_msg = MsgSwapExactAmountInResponse {
        token_out_amount: "1000".to_string(),
    };
    let mut data = Vec::new();
    prost::Message::encode(&resp_msg, &mut data).unwrap();
    let reply_msg = cosmwasm_std::Reply {
        id: SWAP_REPLY_ID,
        result: cosmwasm_std::SubMsgResult::Ok(cosmwasm_std::SubMsgResponse {
            data: Some(cosmwasm_std::Binary::from(data)),
            events: vec![],
        }),
    };
    let resp = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    assert_eq!(resp.messages.len(), 1);
}

#[test]
fn test_proxy_split() {
    let mut deps = mock_dependencies();
    mock_instantiate(&mut deps);

    let routes = vec![SwapAmountInSplitRoute {
        pools: vec![SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        }],
        token_in_amount: "1000".to_string(),
    }];
    let msg = ExecuteMsg::ProxySwapWithFee {
        swap: ProxySwap::SplitRouteSwapExactAmountIn {
            routes,
            token_in_denom: "uion".to_string(),
            token_out_min_amount: Uint128::new(1),
        },
    };
    let info = mock_info("trader", &[Coin::new(1000, "uion")]);
    let resp = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(resp.messages.len(), 2);
    assert_eq!(resp.messages[1].id, SWAP_REPLY_ID);

    let resp_msg = MsgSplitRouteSwapExactAmountInResponse {
        token_out_amount: "1000".to_string(),
    };
    let mut data = Vec::new();
    prost::Message::encode(&resp_msg, &mut data).unwrap();
    let reply_msg = cosmwasm_std::Reply {
        id: SWAP_REPLY_ID,
        result: cosmwasm_std::SubMsgResult::Ok(cosmwasm_std::SubMsgResponse {
            data: Some(cosmwasm_std::Binary::from(data)),
            events: vec![],
        }),
    };
    let resp = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    assert_eq!(resp.messages.len(), 1);
}

#[test]
fn test_min_affiliate_fee_applied_when_rounded_down_single() {
    let mut deps = mock_dependencies();
    // Set affiliate_bps to 30 (0.3%)
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        affiliate_addr: "affiliate".to_string(),
        affiliate_bps: 30,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let routes = vec![SwapAmountInRoute {
        pool_id: 1,
        token_out_denom: "uosmo".to_string(),
    }];
    let exec_msg = ExecuteMsg::ProxySwapWithFee {
        swap: ProxySwap::SwapExactAmountIn {
            routes,
            token_in: Coin::new(1, "uion"),
            token_out_min_amount: Uint128::new(1),
        },
    };
    let info = mock_info("trader", &[Coin::new(1, "uion")]);
    let resp = execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();
    assert_eq!(resp.messages.len(), 1);
    assert_eq!(resp.messages[0].id, 0);
}

#[test]
fn test_min_affiliate_fee_applied_when_rounded_down_split() {
    let mut deps = mock_dependencies();
    // Set affiliate_bps to 30 (0.3%)
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        affiliate_addr: "affiliate".to_string(),
        affiliate_bps: 30,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let routes = vec![SwapAmountInSplitRoute {
        pools: vec![SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        }],
        token_in_amount: "1".to_string(),
    }];
    let exec_msg = ExecuteMsg::ProxySwapWithFee {
        swap: ProxySwap::SplitRouteSwapExactAmountIn {
            routes,
            token_in_denom: "uion".to_string(),
            token_out_min_amount: Uint128::new(1),
        },
    };
    let info = mock_info("trader", &[Coin::new(1, "uion")]);
    let resp = execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();
    // With input-based fee, the min fee (1) fully consumes input. No swap occurs.
    assert_eq!(resp.messages.len(), 1);
    assert_eq!(resp.messages[0].id, 0);
}
