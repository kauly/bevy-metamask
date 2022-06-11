use async_channel::{bounded, Receiver, Sender};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use std::sync::Arc;
use web3::transports::eip_1193;
use web3::types::H160;

pub struct MetaMaskPlugin;
impl Plugin for MetaMaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_comm)
            .add_state(AppState::Ready)
            .add_system(ui_example)
            .add_system_set(
                SystemSet::on_update(AppState::LoadingAddr).with_system(addr_response_system),
            )
            .add_system_set(
                SystemSet::on_update(AppState::LoadingSign).with_system(sign_response_system),
            );
    }
}

struct MetamaskChannel {
    addr_rx: Receiver<H160>,
    addr_tx: Arc<Sender<H160>>,
    sign_rx: Receiver<String>,
    sign_tx: Arc<Sender<String>>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum AppState {
    LoadingAddr,
    LoadingSign,
    Ready,
}

#[derive(Default)]
struct AppData {
    user_wallet_addr: WalletAddress,
    signed: Option<String>,
}

#[derive(Default)]
pub struct WalletAddress(Option<H160>);

fn setup_comm(mut commands: Commands) {
    let (addr_tx, addr_rx) = bounded(1);
    let (sign_tx, sign_rx) = bounded(1);
    commands.insert_resource(MetamaskChannel {
        addr_rx,
        addr_tx: Arc::new(addr_tx),
        sign_rx,
        sign_tx: Arc::new(sign_tx),
    });
    commands.insert_resource(AppData::default());
}

async fn request_account(addr_tx: &Sender<H160>) {
    let provider = eip_1193::Provider::default().unwrap().unwrap();
    let transport = eip_1193::Eip1193::new(provider);
    let web3 = web3::Web3::new(transport);

    let addrs = web3.eth().request_accounts().await.unwrap();

    if !addrs.is_empty() {
        addr_tx.send(addrs[0]).await.unwrap();
    }
}

async fn sign_a_string(sign_tx: &Sender<String>, user_addr: &H160) {
    let provider = eip_1193::Provider::default().unwrap().unwrap();
    let transport = eip_1193::Eip1193::new(provider);
    let web3 = web3::Web3::new(transport);

    let msg = web3::types::Bytes("Sign me".as_bytes().to_vec());
    let result = web3.personal().sign(msg, *user_addr, "").await.unwrap();
    sign_tx.send(result.to_string()).await.unwrap();
}

fn ui_example(
    mut egui_context: ResMut<EguiContext>,
    metamask_ch: ResMut<MetamaskChannel>,
    app_data: Res<AppData>,
    mut app_state: ResMut<State<AppState>>,
) {
    egui::CentralPanel::default().show(egui_context.ctx_mut(), |ui| {
        let addr_tx = metamask_ch.addr_tx.clone();
        let sign_tx = metamask_ch.sign_tx.clone();

        if ui.button("metamask").clicked() {
            app_state.set(AppState::LoadingAddr).unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                request_account(&addr_tx).await;
            });
        }

        if let Some(addr) = &app_data.user_wallet_addr.0 {
            let addr = addr.clone();
            ui.label(addr.to_string());
            if ui.button("Sign a text").clicked() {
                app_state.set(AppState::LoadingSign).unwrap();
                wasm_bindgen_futures::spawn_local(async move {
                    sign_a_string(&sign_tx, &addr).await;
                })
            }
        }

        if let Some(signed) = &app_data.signed {
            ui.label(signed);
        }
    });
}

fn addr_response_system(
    metamask_ch: ResMut<MetamaskChannel>,
    mut app_data: ResMut<AppData>,
    mut app_state: ResMut<State<AppState>>,
) {
    match metamask_ch.addr_rx.try_recv() {
        Ok(addr) => {
            app_data.user_wallet_addr.0 = Some(addr);
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
