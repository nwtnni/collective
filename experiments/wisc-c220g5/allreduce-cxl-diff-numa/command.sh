LD_PRELOAD=./libcollective.so \
    COLLECTIVE_PCI_PATH=/sys/bus/pci/devices/0000:00:05.0/resource2 \
    mpirun \
    -x LD_PRELOAD \
    -x COLLECTIVE_PCI_PATH \
    -H 172.16.0.2,172.16.0.3,172.16.0.4,172.16.0.5,172.16.0.6,172.16.0.7,172.16.0.8,172.16.0.9 \
    --map-by ppr:1:node \
    --mca btl self,tcp \
    --mca btl_tcp_if_include lo,enp0s4 \
    --allow-run-as-root \
    /root/osu_allreduce \
    --full \
    --type mpi_float | tee osu-allreduce-ignore.txt
