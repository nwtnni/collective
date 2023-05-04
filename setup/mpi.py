import subprocess
import sys
import time

import click
from fabric import Connection
from fabric import ThreadingGroup

# ompi_info --param coll tuned --level 5
ALGORITHMS = {
    "allreduce": [
        "ignore",
        "basic-linear",
        "nonoverlapping",
        "recursive-doubling",
        "ring",
        "segmented-ring",
    ],
    "broadcast": [
        "ignore",
        "basic-linear",
        "chain",
        "pipeline",
        "split-binary-tree",
        "binary-tree",
        "binomial-tree",
        "knomial-tree",
        "scatter-allgather",
        "scatter-allgather-ring",
    ],
}


def run(hosts, nodes, interface, benchmark, algorithm):
    print(f"Running {benchmark} {ALGORITHMS[benchmark][algorithm]}...")

    nodes.sudo(f"ethtool -C {interface} stats-block-usecs 100000")

    ifstat_out = f"ifstat-{benchmark}-{ALGORITHMS[benchmark][algorithm]}.txt"
    ifstats = nodes.run(f"~/ifstat -I {interface} -d 120s -i 100ms > {ifstat_out}", asynchronous=True)

    pcm_out = f"pcm-{benchmark}-{ALGORITHMS[benchmark][algorithm]}.txt"
    pcms = nodes.sudo(f"pcm -nc -csv={pcm_out} -i=1200 0.1", asynchronous=True)

    time.sleep(5.0)

    # https://github.com/JiaweiZhuang/aws-mpi-benchmark/tree/master
    command = " ".join([
        "OPAL_PREFIX=/users/nwtnni/openmpi-v5.0.x-install",
        "mpirun",
        "-x OPAL_PREFIX",
        "--map-by ppr:1:node",
        "--mca btl self,tcp",
        f"--mca btl_tcp_if_include {interface}",
        "-H",
        ",".join([host.split('@')[1] for host in hosts]),
        "--mca coll_tuned_use_dynamic_rules 1",
        f"--mca coll_tuned_{'bcast' if benchmark == 'broadcast' else benchmark}_algorithm {algorithm}",
        benchmark,
        "$((2**30))",
    ])

    nodes[0].run(command)

    for ifstat in ifstats.values():
        ifstat.join()
    for pcm in pcms.values():
        pcm.join()

    # Parallel download
    nodes.get(ifstat_out, local=("{host}-" + ifstat_out))
    nodes.get(pcm_out, local=("{host}-" + pcm_out))

    # Serial renames: couldn't find a way to access index from `nodes.get`.
    for index, host in enumerate(hosts):
        subprocess.Popen(["mv", f"{host}-{ifstat_out}", f"{index}-{ifstat_out}"])
        subprocess.Popen(["mv", f"{host}-{pcm_out}", f"{index}-{pcm_out}"])


@click.command()
@click.option("-i", "--interface", required=True)
@click.option("-b", "--benchmark")
@click.option("-a", "--algorithm")
def main(interface, benchmark, algorithm):
    hosts = [host.strip() for host in sys.stdin.readlines()]
    nodes = ThreadingGroup(*hosts, forward_agent=True)

    assert benchmark in ALGORITHMS.keys()

    if algorithm is not None:
        assert benchmark is not None
        try:
            algorithm = int(algorithm)
            assert algorithm >= 0
            assert algorithm < len(ALGORITHMS[benchmark])
        except ValueError:
            assert algorithm in ALGORITHMS[benchmark]
            algorithm = ALGORITHMS[benchmark].index(algorithm)

        run(hosts, nodes, interface, benchmark, algorithm)
        return

    for benchmark in [benchmark] if benchmark is not None else ALGORITHMS.keys():
        for algorithm in range(len(ALGORITHMS[benchmark])):
            run(hosts, nodes, interface, benchmark, algorithm)


if __name__ == "__main__":
    main()
