import sys

from fabric import Connection
from fabric import ThreadingGroup


def download(node):
    node.sudo("apt update", warn=True)
    node.sudo("apt install -y --no-install-recommends iperf libhwloc-common")

    root = "https://github.com/photoszzt/mem_workloads/releases/download/v0.1-alpha-model"
    deps = "deps-install.tar.gz"
    mpi = "openmpi-v5.0.x-install.tar.gz"

    node.run(exists(deps, f"wget {root}/{deps}"))
    node.run(exists(mpi, f"wget {root}/{mpi}"))

    node.run(exists(deps.strip(".tar.gz"), f"tar -xf {deps}"))
    node.run(exists(mpi.strip(".tar.gz"), f"tar -xf {mpi}"))

    root = "https://github.com/nwtnni/collective/releases/download/0.1.0"

    node.run(exists("allreduce", f"wget {root}/allreduce"))
    node.run(exists("broadcast", f"wget {root}/broadcast"))

    node.run(f"chmod 755 allreduce")
    node.run(f"chmod 755 broadcast")


def exists(path, command):
    return f"test -e {path} || {command}"


if __name__ == "__main__":
    hosts = [host.strip() for host in sys.stdin.readlines()]
    group = ThreadingGroup(*hosts)
    download(group)
