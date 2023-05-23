# Environment

## Host

- Disable all cores on node 1
- Disable NMI watchdog
- Disable Intel Turbo
- Disable NUMA balancing
- Set scaling governor to performance

## VM

- 8 VMs
- 2 cores (mapped to 10 cores and 2 threads per core on host)
- No configuration currently--do we also want to change the same kernel settings as on the host?
- Pin VM threads to host cores

## CXL

- 3GB tmpfs file
- 2GB mapped into each VM with QEMU ivshmem device
