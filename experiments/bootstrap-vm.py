import os
import sys

import click
from fabric import Connection

def download(node):
    node.run("git clone git@github.com:photoszzt/vhive_setup.git")
    node.run("./vhive_setup/linux_qemu/download.sh")

    # Temporary workaround for emulab-client-legacy package breaking
    node.sudo("apt remove emulab-client-legacy")

    node.run("./vhive_setup/linux_qemu/install_deps.sh")
    node.run("source ~/mambaforge/bin/activate")

    # Build VM image
    with node.cd("vhive_setup/linux_qemu"):
        node.run("make_vmimg.sh")

    # Set up 2G shared memory file on NUMA node 1
    node.sudo("mkdir /mnt/cxl_mem")
    node.sudo("mount -t tmpfs -o size=4G -o mpol=bind:1 -o rw,nosuid,nodev tmpfs /mnt/cxl_mem")
    node.run("truncate -s 2G /mnt/cxl_mem/mem_1")

    # Build OSU microbenchmarks
    osu = "osu-micro-benchmarks-7.1-1"
    node.sudo("apt install openmpi-bin libopenmpi-dev")
    node.run(f"wget https://mvapich.cse.ohio-state.edu/download/mvapich/{osu}.tar.gz")
    node.run(f"tar -xvf {osu}.tar.gz")
    node.run(f"mkdir {osu}/build")
    with node.cd(f"{osu}/build"):
        node.run("../configure CC=/usr/bin/mpicc CXX=/usr/bin/mpicxx")
        node.run("make -j$(nproc)")

    # TODO: download ivshmem-server, ivshmem_driver.ko from ivshmem repository
    # TODO: download allreduce, broadcast, libcollective.so from collective repository
    # TODO: start ivshmem-server

    with node.cd("vhive_setup/shmem_comm"):
        node.sudo("pip install pyroute2")
        # Expected to fail for now, just get through copying VM images, which is time-consuming
        node.sudo("python3 setup_exp_vm.py --num_vm 8 --shmem_path /tmp/ivshmem_socket", warn=True)


# https://github.com/vtess/Pond/blob/master/cxl-global.sh
def standardize(node):
    # Disable non-maskable interrupt (NMI) watchdog
    # See: https://kernel.googlesource.com/pub/scm/linux/kernel/git/arm64/linux/+/v3.1-rc3/Documentation/nmi_watchdog.txt
    node.sudo("echo 0 | sudo tee /proc/sys/kernel/nmi_watchdog >/dev/null 2>&1")

    # Disable NUMA balancing
    # See: https://www.linux-kvm.org/images/7/75/01x07b-NumaAutobalancing.pdf
    node.sudo(f"echo 0 | sudo tee /proc/sys/kernel/numa_balancing >/dev/null 2>&1")

    system = "/sys/devices/system"

    # Enable all cores
    node.sudo(f"echo 1 | sudo tee {system}/cpu/cpu*/online >/dev/null 2>&1")

    # Enable performance mode
    # See: https://wiki.archlinux.org/title/CPU_frequency_scaling
    node.sudo(f"echo performance | sudo tee {system}/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1")

    # Disable turbo
    # See: https://wiki.archlinux.org/title/CPU_frequency_scaling
    node.sudo(f"echo 1 | sudo tee {system}/cpu/intel_pstate/no_turbo >/dev/null 2>&1")

    # Disable all cores on NUMA node 1
    node.sudo(f"echo 0 | sudo tee {system}/node/node1/cpu*/online >/dev/null 2>&1")


@click.command()
@click.option("-h", "--host", required=True, type=str)
def main(host):
    remote = Connection(host, forward_agent=True)
    download(remote)
    standardize(remote)


if __name__ == "__main__":
    main()
