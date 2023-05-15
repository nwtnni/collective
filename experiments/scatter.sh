#!/usr/bin/env bash
set -euxo pipefail

num_vms="${num_vms:-1}"
dir="${dir:-$(dirname $1)}"

for ((i = 0; i < $num_vms; i++)); do
  ip="172.16.0.$(expr 2 + $i)"
  scp -o StrictHostKeyChecking=false $1 root@$ip:"$dir/$(basename $1)"
done
