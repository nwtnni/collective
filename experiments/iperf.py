import sys
import time

import click
from fabric import ThreadingGroup


@click.command()
@click.option("-u", "--user", required=True)
def main(user):
    hosts = [host.strip() for host in sys.stdin.readlines()]
    nodes = ThreadingGroup(*hosts, user=user)

    # Sanity check
    indices = [int(node.run("hostname -s").stdout.strip().lstrip("node-")) for node in nodes]
    assert(indices == sorted(indices))

    servers = nodes.run("iperf -s", asynchronous=True)
    time.sleep(3.0)

    for i, node in enumerate(nodes):
        for j in range(len(nodes)):
            if i == j:
                continue

            with open(f"{i}-to-{j}.txt", "w") as file:
                file.write(node.run(f"iperf -c 10.0.1.{j + 1}").stdout)

    nodes.run("kill $(ps -A | grep iperf | cut -f3 -d' ')")
    for server in servers.values():
        server.join()


if __name__ == "__main__":
    main()
