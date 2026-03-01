import requests
from time import sleep
import sys

NODE_IPS = [f"172.20.0.{i}" for i in range(2, 12)]  # .2 to .11
PORT = 5000
BASE_URL = "http://{}:{}"
SYNC_WAIT_SECONDS = 60

print("Checking node statuses...")

for ip in NODE_IPS:
    try:
        response = requests.get(BASE_URL.format(ip, PORT) + "/status")
        response.raise_for_status()
    except requests.RequestException as error:
        print(f"{ip}:{PORT} is not responding: {error}")
        print("Did you run `docker compose up` to start the containers?")
        sys.exit(1)

print("All nodes responded successfully.")

print(f"Waiting {SYNC_WAIT_SECONDS} seconds for peers to sync...")
sleep(SYNC_WAIT_SECONDS)

print("Fetching peers...")
response = requests.get(BASE_URL.format(NODE_IPS[0], PORT) + "/peers")
response.raise_for_status()
peers = response.json()

print("Checking peer count...")
assert len(peers) == 10, f"Expected 10 peers, got {len(peers)}"

print("Creating user Bob...")
response = requests.post(
    BASE_URL.format(NODE_IPS[0], PORT) + "/users",
    json={"name": "Bob", "balance": 987},
)
response.raise_for_status()
print(response.text)

print(f"Waiting {SYNC_WAIT_SECONDS} seconds for user creation to propagate...")
sleep(SYNC_WAIT_SECONDS)

print("Transferring 100 coins from Bob to Alice...")
response = requests.post(
    BASE_URL.format(NODE_IPS[0], PORT) + "/transfers",
    json={"from": "Bob", "to": "Alice", "sum": 100},
)
response.raise_for_status()
print(response.text)

print(f"Waiting {SYNC_WAIT_SECONDS} seconds for transfer to propagate...")
sleep(SYNC_WAIT_SECONDS)

print("Fetching users from node01...")
response = requests.get(BASE_URL.format(NODE_IPS[0], PORT) + "/users")
response.raise_for_status()
users = response.json()

balances = {user["name"]: user["balance"] for user in users}

print("Validating balances after transfer...")
assert balances.get("Alice") == 200, f"Alice balance incorrect: expected 200, got {balances.get('Alice')}"
assert balances.get("Bob") == 887, f"Bob balance incorrect: expected 887, got {balances.get('Bob')}"

print("Fetching users from node10 and validating sync...")
response = requests.get(BASE_URL.format(NODE_IPS[-1], PORT) + "/users")
response.raise_for_status()
users = response.json()

balances = {user["name"]: user["balance"] for user in users}

assert balances.get("Alice") == 200, f"Alice balance incorrect: expected 200, got {balances.get('Alice')}"
assert balances.get("Bob") == 887, f"Bob balance incorrect: expected 887, got {balances.get('Bob')}"

print("All checks passed successfully.")
