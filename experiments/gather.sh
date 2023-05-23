#!/usr/bin/env bash
set -euxo pipefail

num_vms="${num_vms:-8}"
dir="${dir:-$(dirname $1)}"

for ((i = 0; i < $num_vms; i++)); do
  ip="172.16.0.$(expr 2 + $i)"
  scp -o StrictHostKeyChecking=false root@$ip:"$1" "$dir/$i-$(basename $1)"
done
