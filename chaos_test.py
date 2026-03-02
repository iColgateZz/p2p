import requests
import subprocess
import time
import random
from collections import defaultdict

class Node:
    def __init__(self, addr):
        self.base_url = "http://" + addr

    def status(self):
        return requests.get(f"{self.base_url}/status", timeout=2).json()

    def create_user(self, name, balance):
        requests.post(
            f"{self.base_url}/users",
            json={"name": name, "balance": balance},
            timeout=2
        )

    def transfer(self, xfrom, to, amount):
        requests.post(
            f"{self.base_url}/transfers",
            json={"from": xfrom, "to": to, "sum": amount},
            timeout=2
        )

    def users(self):
        return requests.get(f"{self.base_url}/users", timeout=2).json()


class LocalNode:
    def __init__(self, port, binary_path="./target/release/p2p"):
        self.port = port
        self.binary_path = binary_path
        self.proc = None
        self.node = Node(f"127.0.0.1:{port}")

    def start(self):
        self.proc = subprocess.Popen(
            [self.binary_path, str(self.port)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    def stop(self):
        self.proc.kill()

def main():
    docker_nodes = [Node(f"172.20.0.{i}:5000") for i in range(2, 12)]
    print(f"[INFO] Tracking {len(docker_nodes)} docker nodes")

    local_nodes = []
    NUM_LOCAL_NODES = 80
    base_port = 5000

    print(f"[INFO] Trying to spawn {NUM_LOCAL_NODES} local nodes")
    for _ in range(NUM_LOCAL_NODES):
        ln = LocalNode(base_port)
        ln.start()
        
        local_nodes.append(ln)
        base_port += 1

    print(f"[INFO] Spawned {len(local_nodes)} local nodes")

    print(f"[INFO] Initializing network starting state")
    # By default, there is Alice with 100 coins
    bootstrap = docker_nodes[0]
    bootstrap.create_user("Bob", 500)
    bootstrap.create_user("Eve", 378)
    
    users = ["Alice", "Bob", "Eve"]
    
    print("[INFO] Waiting for initial propagation")
    time.sleep(10)

    print(f"[INFO] Chaos simulation begins")
    CHAOS_DURATION = 10 * 60        # minutes
    TRANSFER_INTERVAL = 10          # seconds between actions
    SPAWN_PROBABILITY = 0.05       # chance to spawn a node
    KILL_PROBABILITY = 0.05        # chance to kill a local node
    POST_CHAOS_WAIT = 60           # seconds to let network settle

    start_time = time.time()
    while time.time() - start_time < CHAOS_DURATION:
        # Active nodes (docker + alive locals)
        active_nodes = docker_nodes + [ln.node for ln in local_nodes]

        # Random transfer
        sender = random.choice(active_nodes)
        xfrom, to = random.sample(users, 2)
        amount = random.randint(1, 10)

        try:
            print(f"[CHAOS] {xfrom} sent {amount} to {to}")
            sender.transfer(xfrom, to, amount)
        except Exception:
            pass

        # Randomly kill a local node
        if local_nodes and random.random() < KILL_PROBABILITY:
            victim = random.choice(local_nodes)
            print(f"[CHAOS] Killing local node on port {victim.port}")
            victim.stop()
            local_nodes.remove(victim)

        # Randomly spawn a new local node
        if random.random() < SPAWN_PROBABILITY:
            ln = LocalNode(base_port)
            ln.start()
            local_nodes.append(ln)
            print(f"[CHAOS] Spawned new local node on port {base_port}")
            base_port += 1

        time.sleep(TRANSFER_INTERVAL)

    print("[INFO] Chaos phase ended")
    
    print(f"[INFO] Waiting {POST_CHAOS_WAIT}s for network to stabilize")
    time.sleep(POST_CHAOS_WAIT)
    
    print("[INFO] Collecting final state from all nodes")
    all_nodes = docker_nodes + [ln.node for ln in local_nodes]

    chain_heights = defaultdict(int)
    last_hashes = defaultdict(int)
    balances_seen = defaultdict(int)
    
    for node in all_nodes:
        try:
            status = node.status()
            height = status["block_height"]
            last_hash = status["last_block_hash"]

            chain_heights[height] += 1
            last_hashes[last_hash] += 1

            users_state = tuple(
                sorted(
                    (u["name"], u["balance"])
                    for u in node.users()
                )
            )
            balances_seen[users_state] += 1

        except Exception:
            continue
        
    print("\n========== CHAOS TEST RESULTS ==========")
    print(f"Nodes queried          : {len(all_nodes)}")
    print(f"\nBlockchain heights observed: {len(chain_heights)}")
    for height, count in sorted(chain_heights.items()):
        print(f"  Height {height}: {count} node(s)")

    print(f"\nLast block hashes observed: {len(last_hashes)}")
    for h, count in last_hashes.items():
        print(f"  Hash {h[:8]}… : {count} node(s)")

    print(f"\nDistinct balance states: {len(balances_seen)}")
    for i, (state, count) in enumerate(balances_seen.items(), 1):
        print(f"\nState #{i} observed on {count} node(s):")
        for name, balance in state:
            print(f"  {name}: {balance}")

    print("\n========================================")

    for ln in local_nodes:
        ln.stop()


if __name__ == "__main__":
    main()
