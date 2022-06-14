use std::sync::Arc;

use async_channel::{bounded, Receiver, Sender};
use bevy::prelude::*;
use web3::transports::eip_1193;
use web3::types::H160;

pub struct MetaMaskPlugin;
impl Plugin for MetaMaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_comm)
            .add_state(AppState::Ready)
            .add_system_set(
                SystemSet::on_update(AppState::LoadingAddr).with_system(addr_response_system),
            )
            .add_system_set(
                SystemSet::on_update(AppState::LoadingSign).with_system(sign_response_system),
            );
    }
}

pub struct MetamaskChannel {
    addr_rx: Receiver<H160>,
    pub addr_tx: Sender<H160>,
    sign_rx: Receiver<String>,
    pub sign_tx: Sender<String>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    LoadingAddr,
    LoadingSign,
    Ready,
}

#[derive(Default)]
pub struct AppData {
    pub user_wallet_addr: Option<H160>,
    pub signed: Option<String>,
    pub no_metamask: bool,
}

fn setup_comm(mut commands: Commands) {
    let (addr_tx, addr_rx) = bounded(1);
    let (sign_tx, sign_rx) = bounded(1);
    commands.insert_resource(MetamaskChannel {
        addr_rx,
        addr_tx,
        sign_rx,
        sign_tx,
    });

    let provider = eip_1193::Provider::default().unwrap();
    if let Some(_) = provider {
        commands.insert_resource(AppData::default());
    } else {
        commands.insert_resource(AppData {
            no_metamask: true,
            ..AppData::default()
        });
    }
}

pub async fn request_account(addr_tx: &Sender<H160>) {
    let provider = eip_1193::Provider::default().unwrap().unwrap();
    let transport = eip_1193::Eip1193::new(provider);
    let web3 = web3::Web3::new(transport);

    let addrs = web3.eth().request_accounts().await.unwrap();

    if !addrs.is_empty() {
        addr_tx.send(addrs[0]).await.unwrap();
    }
}

pub async fn sign_a_string(sign_tx: &Sender<String>, user_addr: &H160) {
    let provider = eip_1193::Provider::default().unwrap().unwrap();
    let transport = eip_1193::Eip1193::new(provider);
    let web3 = web3::Web3::new(transport);

    let msg = web3::types::Bytes("Sign me".as_bytes().to_vec());
    let result = web3.personal().sign(msg, *user_addr, "").await.unwrap();
    sign_tx.send(result.to_string()).await.unwrap();
}

fn addr_response_system(
    metamask_ch: ResMut<MetamaskChannel>,
    mut app_data: ResMut<AppData>,
    mut app_state: ResMut<State<AppState>>,
) {
    match metamask_ch.addr_rx.try_recv() {
        Ok(addr) => {
            app_data.user_wallet_addr = Some(addr);
            app_state.set(AppState::Ready).unwrap();
        }
        Err(e) => info!("{}", e),
    }
}

fn sign_response_system(
    metamask_ch: ResMut<MetamaskChannel>,
    mut app_data: ResMut<AppData>,
    mut app_state: ResMut<State<AppState>>,
) {
    match metamask_ch.sign_rx.try_recv() {
        Ok(text) => {
            app_data.signed = Some(text);
            app_state.set(AppState::Ready).unwrap();
        }
        Err(e) => info!("{}", e),
    }
}
