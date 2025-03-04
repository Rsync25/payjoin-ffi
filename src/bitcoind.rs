use std::{ sync::{ Mutex, Arc, MutexGuard }, collections::HashMap };

use bitcoin::Amount;
use serde::{ Deserialize, Serialize };
use bitcoincore_rpc::{ self, RpcApi };

use crate::{ CachedOutputs, Input, AddressType };
#[derive(Debug, Deserialize)]
pub struct BitcoindConfig {
    pub rpc_host: String,
    pub cookie: Option<String>,
    pub rpc_user: String,
    pub rpc_pass: String,
    pub cache_dir: String,
}
pub struct BitcoindClient {
    pub config: BitcoindConfig,
    pub bitcoind_mutex: Mutex<bitcoincore_rpc::Client>,
    pub cached_outputs: Arc<Mutex<CachedOutputs>>,
}
impl BitcoindClient {
    pub fn new(config: BitcoindConfig) -> Result<Self, bitcoincore_rpc::Error> {
        let bitcoind = (
            match &config.cookie {
                Some(cookie) =>
                    bitcoincore_rpc::Client::new(
                        &config.rpc_host,
                        bitcoincore_rpc::Auth::CookieFile(cookie.into())
                    ),
                None =>
                    bitcoincore_rpc::Client::new(
                        &config.rpc_host,
                        bitcoincore_rpc::Auth::UserPass(
                            config.rpc_user.clone(),
                            config.rpc_pass.clone()
                        )
                    ),
            }
        ).expect("Failed to connect to bitcoind");
        let seen_input = Arc::new(Mutex::new(CachedOutputs::new(config.cache_dir.clone())?));
        let bitcoind_mutex = Mutex::new(bitcoind);
        Ok(Self { config, bitcoind_mutex, cached_outputs: seen_input })
    }
    fn get_rpc_client(&self) -> MutexGuard<bitcoincore_rpc::Client> {
        return self.bitcoind_mutex.lock().unwrap();
    }
    pub fn load_wallet(&self, wallet_name: String) -> Result<String, bitcoincore_rpc::Error> {
        let client = self.get_rpc_client();
        match client.load_wallet(&wallet_name.as_str()) {
            Ok(e) => {
                return match e.warning {
                    Some(e) => panic!("{:?}", e),
                    None => Ok(wallet_name),
                };
            }
            Err(e) => panic!("{:?}", e),
        };
    }
    pub fn create_wallet(
        &self,
        wallet_name: String,
        disable_private_keys: Option<bool>,
        blank: Option<bool>,
        passphrase: Option<String>,
        avoid_reuse: Option<bool>
    ) -> Result<String, bitcoincore_rpc::Error> {
        let client = self.get_rpc_client();
        match
            client.create_wallet(
                &wallet_name.as_str(),
                disable_private_keys,
                blank,
                passphrase.as_ref().map(|x| x.as_str()),
                avoid_reuse
            )
        {
            Ok(e) => {
                return match e.warning {
                    Some(e) => panic!("{:?}", e),
                    None => Ok(wallet_name),
                };
            }
            Err(e) => panic!("{:?}", e),
        };
    }

    pub fn create_psbt(
        &self,
        inputs: Vec<Input>,
        outputs: HashMap<String, u64>,
        locktime: Option<i64>,
        replaceable: Option<bool>
    ) -> Result<String, bitcoincore_rpc::Error> {
        let client = self.get_rpc_client();
        let pared_inputs = inputs
            .iter()
            .map(|x| x.into())
            .collect::<Vec<bitcoincore_rpc::json::CreateRawTransactionInput>>();
        let parsed_outputs = outputs
            .into_iter()
            .map(|(x, y)| (x, Amount::from_sat(y)))
            .collect::<HashMap<String, Amount>>();
        return match
            client.create_psbt(pared_inputs.as_slice(), &parsed_outputs, locktime, replaceable)
        {
            Ok(e) => Ok(e),
            Err(e) => panic!("{:?}", e),
        };
    }

    pub fn get_new_address(
        &self,
        label: Option<&str>,
        address_type: Option<AddressType>
    ) -> Result<String, bitcoincore_rpc::Error> {
        let client = self.get_rpc_client();
        match
            client.get_new_address(
                label,
                address_type.map(|x| x.into())
            )
        {
            Ok(e) => Ok(e.assume_checked().to_string()),
            Err(e) => panic!("{:?}", e),
        }
    }
}
