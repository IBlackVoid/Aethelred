use rtnetlink::{new_connection, Handle};
use std::net::Ipv4Addr;

const BRIDGE_NAME: &str = "aethel0";
const BRIDGE_IP: Ipv4Addr = Ipv4Addr::new(172, 29, 0, 1);
const PREFIX_LEN: u8 = 24;

pub async fn setup_bridge(handle: &Handle) -> Result<(), rtnetlink::Error> {
    let mut links = handle.link().get().match_name(BRIDGE_NAME.to_string()).execute();
    if links.try_next().await?.is_none() {
        handle.link().add().bridge(BRIDGE_NAME.to_string()).execute().await?;
    }

    let link = handle.link().get().match_name(BRIDGE_NAME.to_string()).execute().try_next().await?.unwrap();
    handle.link().set(link.header.index).up().execute().await?;

    handle.address().add(link.header.index, BRIDGE_IP.into(), PREFIX_LEN).execute().await?;

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
    handle.link().set(peer.header.index).setns_by_pid(container_pid).execute().await?;

    let container_ns_handle = new_connection()?;
    let (conn, new_handle, _) = container_ns_handle;
    tokio::spawn(conn);

    let pid_fd = nix::fcntl::open(format!("/proc/{}/ns/net", container_pid).as_str(), nix::fcntl::OFlag::empty(), nix::sys::stat::Mode::empty())?;
    nix::sched::setns(pid_fd, nix::sched::CloneFlags::CLONE_NEWNET)?;

    new_handle.link().set(peer.header.index).name("eth0".to_string()).execute().await?;
    new_handle.link().set(peer.header.index).up().execute().await?;
    new_handle.address().add(peer.header.index, ip.into(), PREFIX_LEN).execute().await?;

    Ok(())
}