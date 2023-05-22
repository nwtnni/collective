#!/usr/bin/env bash
set -euxo pipefail

num_vms="${num_vms:-8}"

for ((i = 0; i < $num_vms; i++)); do
  ip="172.16.0.$(expr 2 + $i)"
  ssh -o StrictHostKeyChecking=false root@$ip $1
done
