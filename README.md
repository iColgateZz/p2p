
# ITI0215_26 Hajusüsteemid

## Praktikum No 1 – Lihtne P2P süsteem

-------------------------------

## Installeerimine ja jooksutamine

### Repo kloonimine

```bash
git clone https://github.com/iColgateZz/p2p.git
cd p2p
```

### Eeldused

Sõlme jooksutamiseks on minimaalselt vaja _Rust_-i kompilaatorit.
Ametliku paigaldusjuhendi leiate siit: https://doc.rust-lang.org/book/ch01-01-installation.html

### Sõlme käivitamine

Kui kompilaator on paigaldatud, saab sõlme käivitada kahel viisil.

#### Dev mode

```bash
cargo run -- <PORT>
```

#### Production mode

```bash
cargo build --release
./target/release/p2p <PORT>
```

Kui \<PORT> ei ole määratud, siis _by default_ kasutatakse porti 5000.

---

## Süsteemi töö (väga) üldine kirjeldus

Iga sõlm on samal ajal nii klient kui ka server, mis tähendab, et iga sõlm nii küsib teistelt midagi kui ka vastab nende küsimustele. Koos sõlmed moodustavad võrku ning jagavad omavahel teadmisi naabritest, tehingutest ning plokkidest.

Peamised andmestruktuurid igal sõlmel on naabrite list (_peer list_), ootelolevad tehingud (_pending transactions_) ning plokiahel (_blockchain_). 

Kui sõlm liitub võrguga, siis ta alguses üritab kontakti saada nende sõlmedega, mis ta luges `peer_config.json` failist. Nendelt ta saab kiiresti küsida veel naabreid. 

Kohe tema hakkab ka uurima, mis on hetkeseis plokiahelaga. Ta võtab oma viimase ploki _hash_-i ning küsib teistelt, kas on veel _hash_-e, mis tulevad ahelas pärast minu _hash_-i. Kui selliseid on, siis kasutades saadud uusi _hash_-e ta küsib naabritelt puuduolevaid plokke ning ehitab ahela lõpuni.

Ka edaspidi hakkab ta regulaarselt naabritelt küsima nende naabrite kohta ning mis on hetkel võrgus viimane _hash_. Lisaks sellele tegeleb ta enesereklaamiga: iga teatud aja tagant saadab naabritele infot enda _ip_ ning _port_-i kohta.

Iga sõlm on võimeline vastu võtta erinevaid kasutaja tehinguid. Hetkel on toetatud 2 tehingute tüüpi: uue kasutaja loomine ning ülekanne ühelt kasutajalt teisele.

Saadud tehingud saadetakse naabritele lailali ning salvestatakse ootelolevate tehingute listi. Iga teatud aja tagant sõlmed võtavad need tehingud, panevad need uute plokki ning saadavad selle laiali. Niimoodi plokiahel kasvab.

Igal tehingul ning plokil on olemas oma _hash_. Hetkel aga meie neid otseselt mitte kuidagi ei kasuta ehk mingit tehingute ega plokkide sünkroniseerimist ei ole. 

---

## Protokolli kirjeldus

Sõlmed suhtlevad omavahel kasutades _HTTP-protokolli_.  
Hetkel on kasutusel ainult _GET_ ja _POST_ päringud.  

Allpool on kirjeldatud kõik toetatud _endpoint_-id, nende eesmärk ning näidis­päringud ja vastused. 

Eeldatakse, et tehtud päringud on korrektsed, seega vastused nagu `400 Bad request` ja `501 Not implemented`, mis tekivad, kui kasutaja sisestab midagi valesti, on vahele jäetud.

Kui vastuse _status code_ ei ole kirjutatud, siis on see `200 OK`.

---

## 1. `GET /status`

Tagastab sõlme hetkeseisu.
Kasutatakse peamiselt võrgu visualiseerimiseks ja testimiseks.

### Päring

```bash
curl http://127.0.0.1:5000/status
```

### Vastus

```json
{
  "block_height": 5,
  "last_block_hash": "a3f1c9...",
  "pending_txs_num": 2,
  "known_peers": [
    { "ip": "127.0.0.1", "port": 5001 },
    { "ip": "127.0.0.1", "port": 5002 }
  ]
}
```

---

## 2. `GET /peers`

Tagastab juhusliku valiku teadaolevatest naabersõlmedest.
Kasutatakse _peer discovery_ mehhanismis.

### Päring

```bash
curl http://127.0.0.1:5000/peers
```

### Vastus

```json
[
  { "ip": "127.0.0.1", "port": 5001 },
  { "ip": "127.0.0.1", "port": 5002 }
]
```

---

## 3. `POST /peers`

Sõlm reklaamib ennast teistele.

### Päring

```bash
curl -X POST http://127.0.0.1:5000/peers \
  -d '{"ip":"127.0.0.1","port":5003}'
```

### Vastus

```json
{ "message": "Advertisement received" }
```

---

## 4. `GET /hashes`

Tagastab kõik plokiahela plokkide _hash_-id õiges järjekorras.

### Päring

```bash
curl http://127.0.0.1:5000/hashes
```

### Vastus

```json
{
  "hashes": [
    "0000abc...",
    "9f12de...",
    "a3f1c9..."
  ]
}
```

---

## 5. `GET /hashes/{hash}`

Tagastab kõik plokkide _hash_-id, mis tulevad pärast antud _hash_-i.
Kasutatakse plokiahela sünkroniseerimiseks.

### Päring

```bash
curl http://127.0.0.1:5000/hashes/9f12de...
```

### Vastus

```json
{
  "hashes": [
    "a3f1c9...",
    "bb81af..."
  ]
}
```

---

## 6. `GET /blocks/{hash}`

Tagastab konkreetse ploki antud _hash_-i alusel.

### Päring

```bash
curl http://127.0.0.1:5000/blocks/a3f1c9...
```

### Vastus

```json
{
  "hash": "a3f1c9...",
  "prev_hash": "9f12de...",
  "transactions": [
    {
      "hash": "tx1...",
      "data": "Alice=100",
      "timestamp": 1710000000
    }
  ],
  "timestamp": 1710000050
}
```

Kui plokk antud _hash_-iga ei eksisteeri, tagastatakse _404 Not Found_.

---

## 7. `POST /blocks`

Lisab uue ploki ahelasse ja _broadcast_-ib selle teistele sõlmedele.

### Päring

```bash
curl -X POST http://127.0.0.1:5000/blocks \
  -d '{ ... }'
```

### Vastus
`201 Created`
```json
{ "message": "Block accepted" }
```

Kui plokk juba eksisteerib või ei sobi ahelasse:

`200 OK`
```json
{
  "message": "Block already exists or its hash does not match the hash of the last block in the chain"
}
```

---

## 8. `POST /transactions`

Lisab uue tehingu ootelolevate tehingute hulka ja _broadcast_-ib selle võrku.

### Päring

```bash
curl -X POST http://127.0.0.1:5000/transactions \
  -d '{
    "hash": "tx123",
    "data": "Alice->Bob:50",
    "timestamp": 1710000100
  }'
```

### Vastus

`201 Created`
```json
{ "message": "Transaction accepted" }
```

Kui tehing on juba olemas:

`200 OK`
```json
{ "message": "Transaction already exists" }
```

---

## 9. `GET /users`

Tagastab kõik kasutajad ja nende kontosummad arvutatud plokiahela põhjal.

### Päring

```bash
curl http://127.0.0.1:5000/users
```

### Vastus

```json
[
  { "name": "Alice", "balance": 200 },
  { "name": "Bob", "balance": 887 }
]
```

---

## 10. `POST /users`

Loob uue kasutaja ja _broadcast_-ib selle võrku.

### Päring

```bash
curl -X POST http://127.0.0.1:5000/users \
  -d '{"name":"Bob","balance":987}'
```

### Vastus

`201 Created`
```json
{ "message": "User added" }
```

---

## 11. `GET /transfers`

Tagastab kõik plokiahelas toimunud ülekanded.

### Päring

```bash
curl http://127.0.0.1:5000/transfers
```

### Vastus

```json
[
  { "from": "Bob", "to": "Alice", "sum": 100 }
]
```

---

## 12. `POST /transfers`

Lisab uue ülekande tehinguna ja _broadcast_-ib selle võrku.

### Päring

```bash
curl -X POST http://127.0.0.1:5000/transfers \
  -d '{"from":"Bob","to":"Alice","sum":100}'
```

### Vastus

`201 Created`
```json
{ "message": "Transfer accepted" }
```

---
