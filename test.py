import requests
from time import sleep
import sys

PORTS = list(range(5000, 5100))
BASE_URL = "http://127.0.0.1:{}"
SYNC_WAIT_SECONDS = 60

print("Checking node statuses...")

for port in PORTS:
    try:
        response = requests.get(BASE_URL.format(port) + "/status")
        response.raise_for_status()
    except requests.RequestException as error:
        print(f"Port {port} is not responding: {error}")
        print("Did you run test.sh to start the processes?")
        sys.exit(1)

print("All nodes responded successfully.")

print(f"Waiting {SYNC_WAIT_SECONDS} seconds for peers to sync...")
sleep(SYNC_WAIT_SECONDS)

print("Fetching peers...")
response = requests.get(BASE_URL.format(PORTS[0]) + "/peers")
response.raise_for_status()
peers = response.json()

print("Checking peer count...")
assert len(peers) == 100, f"Expected 100 peers, got {len(peers)}"

print("Creating user Bob...")
response = requests.post(
    BASE_URL.format(PORTS[0]) + "/users",
    json={"name": "Bob", "balance": 987},
)
response.raise_for_status()
print(response.text)

print(f"Waiting {SYNC_WAIT_SECONDS} seconds for user creation...")
sleep(SYNC_WAIT_SECONDS)

print("Transferring 100 coins from Bob to Alice...")
response = requests.post(
    BASE_URL.format(PORTS[0]) + "/transfers",
    json={"from": "Bob", "to": "Alice", "sum": 100},
)
response.raise_for_status()
print(response.text)

print(f"Waiting {SYNC_WAIT_SECONDS} seconds for transfer to propagate...")
sleep(SYNC_WAIT_SECONDS)

print("Fetching users...")
response = requests.get(BASE_URL.format(PORTS[0]) + "/users")
response.raise_for_status()
users = response.json()

balances = {user["name"]: user["balance"] for user in users}

print("Validating balances after transfer...")

assert balances.get("Alice") == 200, f"Alice balance incorrect: expected 200, got {balances.get('Alice')}"
assert balances.get("Bob") == 887, f"Bob balance incorrect: expected 887, got {balances.get('Bob')}"

print("Fetching users from another port and validating them...")
response = requests.get(BASE_URL.format(PORTS[30]) + "/users")
response.raise_for_status()
users = response.json()

balances = {user["name"]: user["balance"] for user in users}

assert balances.get("Alice") == 200, f"Alice balance incorrect: expected 200, got {balances.get('Alice')}"
assert balances.get("Bob") == 887, f"Bob balance incorrect: expected 887, got {balances.get('Bob')}"

print("All checks passed successfully.")
