import os
import sys

import click
from fabric import ThreadingGroup

def download(node):
    node.sudo("apt update", warn=True)
    node.sudo("apt install -y --no-install-recommends iperf libhwloc15 pcm")

    root = "https://github.com/photoszzt/mem_workloads/releases/download/v0.1-alpha-model"
    deps = "deps-install.tar.gz"
    mpi = "openmpi-v5.0.x-install.tar.gz"
    osu = "osu-micro-openmpi-install.tar.gz"

    node.run(exists(deps, f"wget {root}/{deps}"))
    node.run(exists(mpi, f"wget {root}/{mpi}"))
    node.run(exists(osu, f"wget {root}/{osu}"))

    deps_dir = deps.strip(".tar.gz")
    mpi_dir = mpi.strip(".tar.gz")
    osu_dir = osu.strip(".tar.gz")

    node.run(exists(deps_dir, f"tar -xf {deps}"))
    node.run(exists(mpi_dir, f"tar -xf {mpi}"))
    node.run(exists(osu_dir, f"tar -xf {osu}"))

    node.sudo(f"rsync -rl {deps_dir}/lib/ /usr/lib/")
    node.sudo(f"rsync -rl {mpi_dir}/lib/ /usr/lib/")
    node.sudo(f"rsync -rl {mpi_dir}/bin/ /usr/bin/")
    node.sudo(f"ldconfig")

    root = "https://github.com/nwtnni/collective/releases/download/0.1.0"

    node.run(exists("allreduce", f"wget {root}/allreduce"))
    node.run(exists("broadcast", f"wget {root}/broadcast"))
    node.run(exists("ifstat", f"wget {root}/ifstat"))

    node.run(f"chmod 755 allreduce")
    node.run(f"chmod 755 broadcast")
    node.run(f"chmod 755 ifstat")

    node.sudo("modprobe msr")


def exists(path, command):
    return f"test -e {path} || {command}"


def keyscan(node, hosts):
    for host in hosts:
        node.run(f"ssh-keyscan -H {host} >> ~/.ssh/known_hosts")


# https://github.com/vtess/Pond/blob/master/cxl-global.sh
def standardize(node, cores):
    # Disable non-maskable interrupt (NMI) watchdog
    # See: https://kernel.googlesource.com/pub/scm/linux/kernel/git/arm64/linux/+/v3.1-rc3/Documentation/nmi_watchdog.txt
    node.sudo("echo 0 | sudo tee /proc/sys/kernel/nmi_watchdog >/dev/null 2>&1")

    system = "/sys/devices/system"

    # Enable all cores
    node.sudo(f"echo 1 | sudo tee {system}/cpu/cpu*/online >/dev/null 2>&1")

    # Enable performance mode
    # See: https://wiki.archlinux.org/title/CPU_frequency_scaling
    node.sudo(f"echo performance | sudo tee {system}/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1")

    # Disable turbo
    # See: https://wiki.archlinux.org/title/CPU_frequency_scaling
    node.sudo(f"echo 1 | sudo tee {system}/cpu/intel_pstate/no_turbo >/dev/null 2>&1")

    # Disable all but two cores
    if cores > 2:
        node.sudo(f"echo 0 | sudo tee {system}/cpu/cpu{{2..{cores - 1}}}/online >/dev/null 2>&1")


@click.command()
@click.option("-u", "--user", required=True)
@click.option("-c", "--cores", required=True, type=int)
def main(user, cores):
    hosts = [host.strip() for host in sys.stdin.readlines()]
    group = ThreadingGroup(*hosts, user=user)
    download(group)
    keyscan(group[0], hosts)
    standardize(group, cores)


if __name__ == "__main__":
    main()
