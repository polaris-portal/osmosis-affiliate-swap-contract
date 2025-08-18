use std::fs::create_dir_all;
use std::path::Path;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use affiliate_swap::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    let out_dir = "schema";
    create_dir_all(out_dir).unwrap();
    remove_schemas(Path::new(out_dir)).unwrap();
    export_schema(&schema_for!(InstantiateMsg), Path::new(out_dir));
    export_schema(&schema_for!(ExecuteMsg), Path::new(out_dir));
    export_schema(&schema_for!(QueryMsg), Path::new(out_dir));
}
