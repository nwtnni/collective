import sys
import time

from fabric import Connection
from fabric import SerialGroup

if __name__ == "__main__":
    hosts = [host.strip() for host in sys.stdin.readlines()]
    nodes = [Connection(host) for host in hosts]

    # Sanity check
    indices = [int(node.run("hostname -s").stdout.strip().lstrip("node-")) for node in nodes]
    assert(indices == sorted(indices))

    servers = [node.run("iperf -s", asynchronous=True) for node in nodes]
    time.sleep(3.0)

    for i, node in enumerate(nodes):
        for j, host in enumerate(hosts):
            if i == j:
                continue

            address = host.split("@")[1]

            with open(f"{i}-to-{j}.txt", "w") as file:
                file.write(node.run(f"iperf -c {address}").stdout)

    for node, server in zip(nodes, servers):
        node.run("kill $(ps -A | grep iperf | cut -f3 -d' ')")
        server.join()
