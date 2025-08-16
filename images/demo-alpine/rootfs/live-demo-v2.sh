#!/bin/sh
# Demo script copied into rootfs
frames=(
"[ 0% ] Initializing core memory.............. 0K"
"[ 5% ] Loading Aethelred Kernel............... OK"
"[ 15%] Spawning daemon threads................ OK"
"[ 30%] Calibrating chaos engines.............. OK"
"[ 45%] Mounting morality inhibitor............ OK"
"[ 60%] Allocating sarcasm buffers............. OK"
"[ 75%] Arming containers...................... OK"
"[ 90%] Igniting hypervisor coils.............. OK"
"[100%] ALL SYSTEMS NOMINAL"
)
for f in "${frames[@]}"; do
  echo "$f"
  sleep 0.05
done 