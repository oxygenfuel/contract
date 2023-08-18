use fuels::accounts::fuel_crypto::SecretKey;
use fuels::types::SizedAsciiString;
use fuels::{prelude::*, types::ContractId};
use rand::prelude::Rng;
use std::str::FromStr;

const RPC: &str = "node-beta-3.fuel.network";
const SECRET_KEY: &str = "49c93db298f06769c8c5a4626b2dd8fb06faae8d1a6974f19616338c3a0a7bee";
const ORDERBOOK_ADDRESS: &str =
    "0xfe90a3162e3d42250f2d628c3f558ca59bda4d657ed9641c647e75e3500cc74d";

const USDC: &str = "0x8f5aa42d45905e5c5f1d7900783d978e71a408fc7d8588c366d9775ff00363cc";
const BTC: &str = "0x5c6003d3777dfc27e9a438e6616da10c6b30274eb4e6fe28f0c75cb2abe39683";
const ETH: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

abigen!(
    Contract(name = "Orderbook", abi = "out/debug/orderbook-abi.json"),
    Contract(
        name = "TokenContract",
        abi = "../token/out/debug/token_contract-abi.json"
    )
);

async fn init(instance: &Orderbook<WalletUnlocked>, base: ContractId, quote: ContractId) {
    instance
        .methods()
        .init(base, quote)
        .tx_params(TxParameters::default().set_gas_price(1))
        .call()
        .await
        .unwrap();
}

async fn place_order(
    id: Bech32ContractId,
    wallet: &WalletUnlocked,
    order: PlaceOrder,
) -> Vec<Match> {
    let orderbook = Orderbook::new(id, wallet.clone());
    let ret = orderbook
        .methods()
        .place_order(order.clone())
        .tx_params(TxParameters::default().set_gas_price(1))
        .call()
        .await
        .unwrap();
    ret.value
}

async fn orderbook_view(instance: &Orderbook<WalletUnlocked>) {
    let orderbook = instance.methods().orderbook(0).call().await.unwrap();
    println!("   {:?}", orderbook.value);
    let orderbook = instance.methods().orderbook(1).call().await.unwrap();
    println!("   {:?}", orderbook.value);
}

pub async fn get_orderbook_instance(wallet: &WalletUnlocked) -> Orderbook<WalletUnlocked> {
    let id = Contract::load_from("./out/debug/orderbook.bin", LoadConfiguration::default())
        .unwrap()
        .deploy(wallet, TxParameters::default())
        .await
        .unwrap();

    let instance = Orderbook::new(id, wallet.clone());
    instance
}

async fn deposit_asset(
    asset_contract_id: Bech32ContractId,
    orderbook_contract_id: Bech32ContractId,
    wallet: &WalletUnlocked,
    amount: u64,
) {
    let token_instance = TokenContract::new(asset_contract_id, wallet.clone());
    let orderbook = Orderbook::new(orderbook_contract_id, wallet.clone());
    let asset_id = AssetId::from(*token_instance.contract_id().hash());
    let deposit_amount = parse_units(amount, 9);
    // println!("====>deposit_asset :{:?}", deposit_amount);
    let call_params = CallParameters::new(deposit_amount, asset_id, 1_000_000);
    orderbook
        .methods()
        .deposit()
        .call_params(call_params)
        .unwrap()
        .call()
        .await
        .unwrap();
}

async fn asset_balance(
    contract_id: Bech32ContractId,
    contract_asset_id: ContractId,
    wallet: WalletUnlocked,
) -> u64 {
    let orderbook = Orderbook::new(contract_id, wallet.clone());
    let balance = orderbook
        .methods()
        .balance(Address::from(wallet.address()), contract_asset_id)
        .simulate()
        .await
        .unwrap();

    println!(
        "wallet: {:?}, asset: {:?}, balance: {:?}",
        Address::from(wallet.address()),
        contract_asset_id,
        balance.value
    );

    balance.value
}

#[tokio::test]
async fn test_place_order() {


}

// #[tokio::test]
async fn test_taker_buy() {
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(
            Some(2),             /* Single wallet */
            Some(2),             /* Single coin (UTXO) */
            Some(1_000_000_000), /* Amount per coin */
        ),
        None,
        None,
    )
    .await;
    let taker_wallet = wallets.pop().unwrap();
    let maker_wallet = wallets.pop().unwrap();

    println!("maker:{:?}", maker_wallet.address());
    println!("taker:{:?}", taker_wallet.address());
    //--------------- DEPLOY TOKEN ---------------
    let usdc_config = DeployTokenConfig {
        name: String::from("USD Coin"),
        symbol: String::from("USDC"),
        decimals: 9,
        mint_amount: 1 * 1_000_000_000,
    };
    let quote_token_instance = get_token_contract_instance(&taker_wallet, usdc_config).await;

    println!("wallet balance:");
    print_balances(&maker_wallet).await;
    print_balances(&taker_wallet).await;
    let orderbook = get_orderbook_instance(&taker_wallet).await;
    let base_contract_id = ContractId::from_str(ETH).unwrap();
    let quote_contract_id = ContractId::from(quote_token_instance.contract_id());
    init(&orderbook, base_contract_id, quote_contract_id).await;
    deposit_asset(
        quote_token_instance.contract_id().clone(),
        orderbook.contract_id().clone(),
        &taker_wallet,
        1000,
    )
    .await;

    deposit_asset(
        Bech32ContractId::from(base_contract_id),
        orderbook.contract_id().clone(),
        &maker_wallet,
        1,
    )
    .await;

    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        maker_wallet.clone(),
    )
    .await;
    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        maker_wallet.clone(),
    )
    .await;

    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        taker_wallet.clone(),
    )
    .await;
    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        taker_wallet.clone(),
    )
    .await;

    let maker_orders = vec![PlaceOrder {
        amount: 100000000, // 0.1
        price: 10000000,   // 0.01
        order_side: 1,
    }];

    for order in maker_orders.iter() {
        place_order(
            orderbook.contract_id().clone(),
            &maker_wallet,
            order.clone(),
        )
        .await;
        // orderbook_view(&orderbook).await;
    }
    orderbook_view(&orderbook).await;

    let matches_ = place_order(
        orderbook.contract_id().clone(),
        &taker_wallet.clone(),
        PlaceOrder {
            amount: 100000000, // 0.1
            price: 10000000,   // 0.01
            order_side: 0,     // buy
        },
    )
    .await;
    println!(" {:?}", matches_);
    orderbook_view(&orderbook).await;

    let maker_base_balances = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        maker_wallet.clone(),
    )
    .await;
    let maker_quote_balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        maker_wallet.clone(),
    )
    .await;
    let taker_base_balance = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        taker_wallet.clone(),
    )
    .await;
    let taker_quote_balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        taker_wallet.clone(),
    )
    .await;

    // assert_eq!(maker_base_balances, 99999998);
    // assert_eq!(maker_quote_balance, 8);
    // assert_eq!(taker_base_balance, 2);
    // assert_eq!(taker_quote_balance, 99999992);
}

#[tokio::test]
// maker buy eth , taker sell eth
async fn test_taker_sell() {
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(
            Some(2),             /* Single wallet */
            Some(2),             /* Single coin (UTXO) */
            Some(1_000_000_000), /* Amount per coin */
        ),
        None,
        None,
    )
    .await;
    let taker_wallet = wallets.pop().unwrap();
    let maker_wallet = wallets.pop().unwrap();
    //--------------- DEPLOY TOKEN ---------------
    let usdc_config = DeployTokenConfig {
        name: String::from("USD Coin"),
        symbol: String::from("USDC"),
        decimals: 9,
        mint_amount: 10000,
    };
    let quote_token_instance = get_token_contract_instance(&maker_wallet, usdc_config).await;

    println!("wallet balance:");
    print_balances(&maker_wallet).await;
    print_balances(&taker_wallet).await;
    let base_contract_id = ContractId::from_str(ETH).unwrap();
    let orderbook = get_orderbook_instance(&taker_wallet).await;
    let quote_contract_id = ContractId::from(quote_token_instance.contract_id());
    init(&orderbook, base_contract_id, quote_contract_id).await;

    // maker deposit usdc
    deposit_asset(
        quote_token_instance.contract_id().clone(),
        orderbook.contract_id().clone(),
        &maker_wallet,
        1000,
    )
    .await;

    // taker deposit eth
    deposit_asset(
        Bech32ContractId::from(base_contract_id),
        orderbook.contract_id().clone(),
        &taker_wallet,
        1,
    )
    .await;

    println!("maker contract balance:");
    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        maker_wallet.clone(),
    )
    .await;
    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        maker_wallet.clone(),
    )
    .await;

    println!("taker contract balance:");
    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        taker_wallet.clone(),
    )
    .await;
    let _balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        taker_wallet.clone(),
    )
    .await;

    // makers place orders
    let maker_orders = vec![
        PlaceOrder {
            price: (0.01 * 1_000_000_000 as f64) as u64,
            amount: 80 * 1_000_000_000,
            order_side: 0,
        },
        PlaceOrder {
            price: (0.01 * 1_000_000_000 as f64) as u64,
            amount: 80 * 1_000_000_000,
            order_side: 0,
        },
    ];

    println!("place maker order");
    for order in maker_orders.iter() {
        place_order(
            orderbook.contract_id().clone(),
            &maker_wallet,
            order.clone(),
        )
        .await;
        // orderbook_view(&orderbook).await;
    }
    orderbook_view(&orderbook).await;

    println!("place taker order");
    // taker place order
    let matches_ = place_order(
        orderbook.contract_id().clone(),
        &taker_wallet.clone(),
        PlaceOrder {
            price: 70 * 1_000_000_000,
            amount: (0.02 * 1_000_000_000 as f64) as u64,
            order_side: 1, // sell
        },
    )
    .await;
    println!(" {:?}", matches_);
    orderbook_view(&orderbook).await;

    let maker_base_balances = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        maker_wallet.clone(),
    )
    .await;
    let maker_quote_balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        maker_wallet.clone(),
    )
    .await;
    let taker_base_balance = asset_balance(
        orderbook.contract_id().clone(),
        base_contract_id,
        taker_wallet.clone(),
    )
    .await;
    let taker_quote_balance = asset_balance(
        orderbook.contract_id().clone(),
        quote_contract_id,
        taker_wallet.clone(),
    )
    .await;

    // assert_eq!(maker_base_balances, 99999998);
    // assert_eq!(maker_quote_balance, 8);
    // assert_eq!(taker_base_balance, 2);
    // assert_eq!(taker_quote_balance, 99999992);
}

pub struct DeployTokenConfig {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub mint_amount: u64,
}

pub async fn get_token_contract_instance(
    wallet: &WalletUnlocked,
    mut deploy_config: DeployTokenConfig,
) -> TokenContract<WalletUnlocked> {
    let mut name = deploy_config.name.clone();
    let mut symbol = deploy_config.symbol.clone();
    let decimals = deploy_config.decimals;

    deploy_config
        .name
        .push_str(" ".repeat(32 - deploy_config.name.len()).as_str());
    deploy_config
        .symbol
        .push_str(" ".repeat(8 - deploy_config.symbol.len()).as_str());

    let id = Contract::load_from(
        "./out/debug/token_contract.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(wallet, TxParameters::default())
    .await
    .unwrap();

    let mint_amount = parse_units(deploy_config.mint_amount, decimals);
    name.push_str(" ".repeat(32 - deploy_config.name.len()).as_str());
    symbol.push_str(" ".repeat(8 - deploy_config.symbol.len()).as_str());

    let instance = TokenContract::new(id, wallet.clone());
    let methods = instance.methods();

    let config: TokenInitializeConfig = TokenInitializeConfig {
        name: SizedAsciiString::<32>::new(deploy_config.name).unwrap(),
        symbol: SizedAsciiString::<8>::new(deploy_config.symbol).unwrap(),
        decimals: deploy_config.decimals,
    };

    let _res = methods
        .initialize(config, mint_amount, Address::from(wallet.address()))
        .call()
        .await;
    let _res = methods.mint().append_variable_outputs(1).call().await;

    instance
}

pub fn parse_units(num: u64, decimals: u8) -> u64 {
    num * 10u64.pow(decimals as u32)
}

pub fn format_units(num: u64, decimals: u8) -> u64 {
    num / 10u64.pow(decimals as u32)
}

pub async fn print_balances(wallet: &WalletUnlocked) {
    let balances = wallet.get_balances().await.unwrap();
    println!("{:?}  {:#?}\n", Address::from(wallet.address()), balances);
}

pub async fn setup() -> (WalletUnlocked, WalletUnlocked, Bech32ContractId) {
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(
            Some(1),             /* Single wallet */
            Some(1),             /* Single coin (UTXO) */
            Some(1_000_000_000), /* Amount per coin */
        ),
        None,
        None,
    )
    .await;

    let maker = wallets.pop().unwrap();
    let taker = wallets.pop().unwrap();

    let orderbook_id =
        Contract::load_from("./out/debug/orderbook.bin", LoadConfiguration::default())
            .unwrap()
            .deploy(&maker, TxParameters::default())
            .await
            .unwrap();
    (maker, taker, orderbook_id)
}

async fn deploy_orderbook(deploy_config: DeployOrderbookConfig) {
    // Create a provider pointing to the testnet.
    let provider = match Provider::connect("beta-3.fuel.network").await {
        Ok(p) => p,
        Err(error) => panic!("❌ Problem creating provider: {:#?}", error),
    };

    // Setup a private key
    let secret = SecretKey::from_str(SECRET_KEY).unwrap();
    // Create the wallet.
    let wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));

    // Optional: Configure deployment parameters
    let tx_parameters = TxParameters::default()
        .set_gas_price(1)
        .set_gas_limit(10000000)
        .set_maturity(0);

    let mut rng = rand::thread_rng();
    let salt = rng.gen::<[u8; 32]>();
    let id = Contract::load_from(
        "./out/debug/orderbook.bin",
        LoadConfiguration::default().set_salt(salt),
    )
    .unwrap()
    .deploy(&wallet, tx_parameters)
    .await
    .unwrap();

    // println!("Contract deployed @ {id}");
    let bech32_address = Bech32Address::from_str(&id.to_string())
        .expect("failed to create Bech32 address from string");

    println!(
        "Orderbook {:?}-{:?}",
        deploy_config.base_name, deploy_config.quote_name
    );
    println!("Contract deployed @ {bech32_address}");
    // Convert to Address
    let plain_address: Address = bech32_address.into();
    println!("Contract _plain_address @ 0x{plain_address}");
    let base_contract_id = ContractId::from_str(&deploy_config.base_asset).unwrap();
    let quote_contract_id = ContractId::from_str(&deploy_config.quote_asset).unwrap();

    let instance = Orderbook::new(id, wallet.clone());
    instance
        .methods()
        .init(base_contract_id, quote_contract_id)
        .tx_params(TxParameters::default().set_gas_price(1))
        .call()
        .await
        .unwrap();
}

#[derive(Debug)]
struct DeployOrderbookConfig {
    base_asset: String,
    quote_asset: String,
    base_name: String,
    quote_name: String,
}

#[tokio::test]
async fn deploy() {
    // YOUR TOKENS ARRAY HERE
    let configs: Vec<DeployOrderbookConfig> = vec![
        DeployOrderbookConfig {
            base_asset: String::from(ETH),
            quote_asset: String::from(USDC),
            base_name: "ETH".to_string(),
            quote_name: "USDC".to_string(),
        },
        // DeployOrderbookConfig {
        //     base_asset: String::from(BTC),
        //     quote_asset: String::from(USDC),
        //     base_name: "BTC".to_string(),
        //     quote_name: "USDC".to_string(),
        // },
    ];

    for config in configs {
        deploy_orderbook(config).await;
    }
}
