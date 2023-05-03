from fabric import Connection
from fabric import task
from fabric import ThreadingGroup


@task
def download(node):
    node.sudo("apt update", warn=True)
    node.sudo("apt install libhwloc-common")

    root = "https://github.com/photoszzt/mem_workloads/releases/download/v0.1-alpha-model"
    deps = "deps-install.tar.gz"
    mpi = "openmpi-v5.0.x-install.tar.gz"

    if not exists(node, deps):
        node.run(f"wget {root}/{deps}")
    if not exists(node, mpi):
        node.run(f"wget {root}/{mpi}")

    if not exists(node, deps.strip(".tar.gz")):
        node.run(f"tar -xf {deps}")
    if not exists(node, mpi.strip(".tar.gz")):
        node.run(f"tar -xf {mpi}")

    node.run(f"rm -f {deps}")
    node.run(f"rm -f {mpi}")

    root = "https://github.com/nwtnni/collective/releases/download/0.1.0"

    if not exists(node, "allreduce"):
        node.run(f"wget {root}/allreduce")
    if not exists(node, "broadcast"):
        node.run(f"wget {root}/broadcast")

    node.run(f"chmod 755 allreduce")
    node.run(f"chmod 755 broadcast")

def exists(node, path):
    return not node.run(f"test -f {path}", warn=True).failed
