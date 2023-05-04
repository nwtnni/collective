import sys
import time

import click
from fabric import ThreadingGroup


@click.command()
@click.option("-u", "--user", required=True)
def main(user):
    hosts = [host.strip() for host in sys.stdin.readlines()]
    nodes = ThreadingGroup(host, user=user)

    # Sanity check
    indices = [int(node.run("hostname -s").stdout.strip().lstrip("node-")) for node in nodes]
    assert(indices == sorted(indices))

    servers = node.run("iperf -s", asynchronous=True)
    time.sleep(3.0)

    for i, node in enumerate(nodes):
        for j, host in enumerate(hosts):
            if i == j:
                continue

            with open(f"{i}-to-{j}.txt", "w") as file:
                file.write(node.run(f"iperf -c {host}").stdout)

    nodes.run("kill $(ps -A | grep iperf | cut -f3 -d' ')")
    for server in servers.values():
        server.join()


if __name__ == "__main__":
    main()
