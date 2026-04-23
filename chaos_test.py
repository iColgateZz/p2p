import requests
import subprocess
import time
import random
from collections import defaultdict


class Node:
    def __init__(self, addr):
        self.base_url = "http://" + addr

    def status(self):
        return requests.get(f"{self.base_url}/status", timeout=5).json()

    def create_user(self, name, balance):
        requests.post(
            f"{self.base_url}/users",
            json={"name": name, "balance": balance},
            timeout=5
        ).raise_for_status()

    def transfer(self, xfrom, to, amount):
        requests.post(
            f"{self.base_url}/transfers",
            json={"from": xfrom, "to": to, "sum": amount},
            timeout=5
        ).raise_for_status()

    def users(self):
        return requests.get(f"{self.base_url}/users", timeout=5).json()


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
    bootstrap = LocalNode(5000)
    bootstrap.start()
    print("[INFO] Using localhost:5000 as bootstrap node (never killed)")

    local_nodes = []
    NUM_LOCAL_NODES = 50
    base_port = 5001

    print(f"[INFO] Trying to spawn {NUM_LOCAL_NODES} local nodes")
    for _ in range(NUM_LOCAL_NODES):
        ln = LocalNode(base_port)
        ln.start()
        local_nodes.append(ln)
        base_port += 1

    print(f"[INFO] Spawned {len(local_nodes)} local nodes")

    print("[INFO] Initializing network starting state")
    time.sleep(5)

    expected_balances = {
        "Alice": 100,
    }

    try:
        bootstrap.node.create_user("Bob", 500)
        expected_balances["Bob"] = 500
    except Exception as e:
        print(f"[WARN] Failed to create Bob: {e}")

    try:
        bootstrap.node.create_user("Eve", 378)
        expected_balances["Eve"] = 378
    except Exception as e:
        print(f"[WARN] Failed to create Eve: {e}")

    users = list(expected_balances.keys())
    successful_transfers = 0
    failed_transfers = 0

    print("[INFO] Waiting for initial propagation")
    time.sleep(10)

    print(f"[INFO] Chaos simulation begins")
    CHAOS_DURATION = 10 * 60        # minutes
    TRANSFER_INTERVAL = 0.2          # seconds between actions
    SPAWN_PROBABILITY = 0.05       # chance to spawn a node
    KILL_PROBABILITY = 0.05        # chance to kill a local node
    POST_CHAOS_WAIT = 300           # seconds to let network settlex

    start_time = time.time()
    while time.time() - start_time < CHAOS_DURATION:
        active_nodes = [bootstrap.node] + [ln.node for ln in local_nodes]

        sender = random.choice(active_nodes)
        xfrom, to = random.sample(users, 2)
        amount = random.randint(1, 10)

        try:
            sender.transfer(xfrom, to, amount)
            expected_balances[xfrom] -= amount
            expected_balances[to] += amount
            successful_transfers += 1
            print(f"[CHAOS] {xfrom} sent {amount} to {to}")
        except Exception as e:
            failed_transfers += 1

        if local_nodes and random.random() < KILL_PROBABILITY:
            victim = random.choice(local_nodes)
            print(f"[CHAOS] Killing local node on port {victim.port}")
            victim.stop()
            local_nodes.remove(victim)

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
    all_nodes = [bootstrap.node] + [ln.node for ln in local_nodes]

    chain_heights = defaultdict(int)
    last_hashes = defaultdict(int)
    balances_seen = defaultdict(int)

    expected_state = tuple(sorted(expected_balances.items()))
    exact_match_nodes = 0

    for node in all_nodes:
        try:
            status = node.status()
            height = status["block_height"]
            last_hash = status["last_block_hash"]

            chain_heights[height] += 1
            last_hashes[last_hash] += 1

            users_state = tuple(sorted((u["name"], u["balance"]) for u in node.users()))
            balances_seen[users_state] += 1

            if users_state == expected_state:
                exact_match_nodes += 1


        except Exception:
            continue
        
    print("\nParams:")
    print(f"NUM_LOCAL_NODES     : {NUM_LOCAL_NODES}")
    print(f"CHAOS_DURATION      : {CHAOS_DURATION}")
    print(f"TRANSFER_INTERVAL   : {TRANSFER_INTERVAL}")
    print(f"SPAWN_PROBABILITY   : {SPAWN_PROBABILITY}")
    print(f"KILL_PROBABILITY    : {KILL_PROBABILITY}")
    print(f"POST_CHAOS_WAIT     : {POST_CHAOS_WAIT}")

    print("\n========== CHAOS TEST RESULTS ==========")
    print(f"Nodes queried          : {len(all_nodes)}")
    print(f"Successful transfers   : {successful_transfers}")
    print(f"Failed transfers       : {failed_transfers}")

    print("\nExpected final balances (source of truth):")
    for name, balance in expected_state:
        print(f"  {name}: {balance}")

    print(f"\nBlockchain heights observed: {len(chain_heights)}")
    for height, count in sorted(chain_heights.items()):
        print(f"  Height {height}: {count} node(s)")

    print(f"\nLast block hashes observed: {len(last_hashes)}")
    for h, count in last_hashes.items():
        print(f"  Hash {h[:8]}… : {count} node(s)")

    print(f"\nDistinct balance states: {len(balances_seen)}")
    for i, (state, count) in enumerate(balances_seen.items(), 1):
        verdict = " <-- EXPECTED" if state == expected_state else ""
        print(f"\nState #{i} observed on {count} node(s):{verdict}")
        for name, balance in state:
            print(f"  {name}: {balance}")

    print(f"\nNodes matching expected state: {exact_match_nodes}/{len(all_nodes)}")
    print("\n========================================")

    bootstrap.stop()
    for ln in local_nodes:
        ln.stop()


if __name__ == "__main__":
    main()
