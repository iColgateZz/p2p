#!/bin/bash

cargo build --release
killall p2p 2>/dev/null
sleep 1

for i in {5000..5099}
do
    target/release/p2p "$i" > /dev/null 2>&1 &
done

echo "Spawned 100 processes in the background"
echo "If you wish to kill the processes, run killall p2p"