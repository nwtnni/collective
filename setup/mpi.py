import subprocess
import sys
import time

from fabric import Connection
from fabric import ThreadingGroup


if __name__ == "__main__":
    interface = None

    try:
        interface = sys.argv[1]
    except IndexError:
        print("Usage: python mpi.py <NETWORK_INTERFACE>")
        exit(1)

    hosts = [host.strip() for host in sys.stdin.readlines()]
    nodes = [Connection(host, forward_agent=True) for host in hosts]

    ifstats = [node.run(f"~/ifstat -I {interface} -i 20ms -d 15s > ifstat-broadcast.txt", asynchronous=True) for node in nodes]
    pcms = [node.sudo("timeout 15s pcm-memory -nc -s -csv=pcm-memory-broadcast.txt 0.02", asynchronous=True, warn=True) for node in nodes]

    time.sleep(1.0)

    # https://github.com/JiaweiZhuang/aws-mpi-benchmark/tree/master
    # for algorithm in range(10):
    command = " ".join([
        "OPAL_PREFIX=/users/nwtnni/openmpi-v5.0.x-install",
        "mpirun",
        "-x OPAL_PREFIX",
        "--map-by ppr:1:node",
        "--mca btl self,tcp",
        f"--mca btl_tcp_if_include {interface}",
        "-H",
        ",".join([host.split('@')[1] for host in hosts]),
        # "--mca coll_tuned_use_dynamic_rules 1",
        # f"--mca coll_tuned_bcast_algorithm {algorithm}",
        "broadcast",
        "$((2**30))",
    ])

    nodes[0].run(command)

    for ifstat in ifstats:
        ifstat.join()
    for pcm in pcms:
        pcm.join()

    for index, host in enumerate(hosts):
        subprocess.run(["scp", f"{host}:~/ifstat-broadcast.txt", f"{index}-ifstat-broadcast.txt"])
        subprocess.run(["scp", f"{host}:~/pcm-memory-broadcast.txt", f"{index}-pcm-memory-broadcast.txt"])
