# Allreduce

Dimensions:

- Node type
  - CXL emulation (`c`) (1 host, 8 VMs, two cores each, QEMU `ivshmem`)
  - Physical (`p`) machines (8 hosts, all but two cores disabled)
  - Virtual (`v`) machines (1 host, 8 VMs, two cores each)

For CXL emulation,

- Shared memory allocated on same NUMA (`s`) or different NUMA (`d`)
- Coordination using spinlocks (`l`) or interrupts (`i`)
- Algorithm using multiple (`m`) or single (`s`) buffers

# Broadcast CXL

Dimensions:

- Shared memory allocated on same NUMA (`s`) or different NUMA (`d`)
