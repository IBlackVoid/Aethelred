pub mod error;

pub mod proto {
    tonic::include_proto!("aethel");
}

#[macro_export]
macro_rules! syscall {
    ($syscall:expr) => {
        $syscall.map_err(|e| {
            AethelError::Nix(e)
        })
    };
}