import sys

import click
from fabric import ThreadingGroup

@click.command()
@click.option("-u", "--user", required=True)
@click.argument("command")
def main(user, command):
    hosts = [host.strip() for host in sys.stdin.readlines()]
    group = ThreadingGroup(*hosts, user=user)
    group.run(command)


if __name__ == "__main__":
    main()
