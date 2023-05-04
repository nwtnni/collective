import sys

from fabric import Connection
from fabric import ThreadingGroup


if __name__ == "__main__":
    hosts = [host.strip() for host in sys.stdin.readlines()]
    root = Connection(hosts[0], forward_agent=True)

    # https://github.com/JiaweiZhuang/aws-mpi-benchmark/tree/master
    # for algorithm in range(10):
    command = " ".join([
        "OPAL_PREFIX=/users/nwtnni/openmpi-v5.0.x-install",
        "mpirun",
        "-x OPAL_PREFIX",
        "--map-by ppr:1:node",
        "--mca btl self,tcp",
        "-H",
        ",".join([host.split('@')[1] for host in hosts]),
        # "--mca coll_tuned_use_dynamic_rules 1",
        # f"--mca coll_tuned_bcast_algorithm {algorithm}",
        "broadcast",
        "$((2**30))",
    ])

    root.run(command)
