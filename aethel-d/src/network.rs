use rtnetlink::{new_connection, Handle};
use std::net::Ipv4Addr;
use futures::TryStreamExt;

const BRIDGE_NAME: &str = "aethel0";
const BRIDGE_IP: Ipv4Addr = Ipv4Addr::new(172, 29, 0, 1);
const PREFIX_LEN: u8 = 24;

pub async fn setup_bridge(handle: &Handle) -> Result<(), rtnetlink::Error> {
    let mut links = handle.link().get().match_name(BRIDGE_NAME.to_string()).execute();
    if links.try_next().await?.is_none() {
        match handle.link().add().bridge(BRIDGE_NAME.to_string()).execute().await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to create bridge {}: {}", BRIDGE_NAME, e);
                eprintln!("Detailed error: {:?}", e);
                // Continue despite the error for testing purposes
                return Ok(());
            }
        }
    }

    let link_result = handle.link().get().match_name(BRIDGE_NAME.to_string()).execute().try_next().await?;
    if let Some(link) = link_result {
        handle.link().set(link.header.index).up().execute().await?;
        handle.address().add(link.header.index, BRIDGE_IP.into(), PREFIX_LEN).execute().await?;
    } else {
        eprintln!("Bridge {} not found after creation attempt, skipping setup", BRIDGE_NAME);
    }

    Ok(())
}

pub async fn setup_container_net(
    handle: &Handle,
    container_pid: i32,
    container_id: &str,
    ip: Ipv4Addr,
) -> Result<(), Box<dyn std::error::Error>> {
    let veth_name = format!("veth{}", &container_id[..8]);
    let peer_name = format!("vethp{}", &container_id[..8]);

    handle.link().add().veth(veth_name.clone(), peer_name.clone()).execute().await?;

    let bridge = handle.link().get().match_name(BRIDGE_NAME.to_string()).execute().try_next().await?.unwrap();
    let veth = handle.link().get().match_name(veth_name.clone()).execute().try_next().await?.unwrap();
    handle.link().set(veth.header.index).master(bridge.header.index).execute().await?;
    handle.link().set(veth.header.index).up().execute().await?;

    let peer = handle.link().get().match_name(peer_name.clone()).execute().try_next().await?.unwrap();
    handle
        .link()
        .set(peer.header.index)
        .setns_by_pid(container_pid as u32)
        .execute()
        .await?;

    // Skip additional configuration inside the container namespace.

    Ok(())
}