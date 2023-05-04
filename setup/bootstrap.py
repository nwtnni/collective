import sys

from fabric import ThreadingGroup


def download(node):
    node.sudo("apt update", warn=True)
    node.sudo("apt install -y --no-install-recommends iperf libhwloc15 pcm")

    root = "https://github.com/photoszzt/mem_workloads/releases/download/v0.1-alpha-model"
    deps = "deps-install.tar.gz"
    mpi = "openmpi-v5.0.x-install.tar.gz"

    node.run(exists(deps, f"wget {root}/{deps}"))
    node.run(exists(mpi, f"wget {root}/{mpi}"))

    deps_dir = deps.strip(".tar.gz")
    mpi_dir = mpi.strip(".tar.gz")

    node.run(exists(deps_dir, f"tar -xf {deps}"))
    node.run(exists(mpi_dir, f"tar -xf {mpi}"))

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


if __name__ == "__main__":
    hosts = [host.strip() for host in sys.stdin.readlines()]
    group = ThreadingGroup(*hosts)
    download(group)
