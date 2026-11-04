#!/usr/bin/env bash
# End-to-end smoke test of the generated hexagonal API against live PostgreSQL.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
cd /home/z/my-project/download/hex-arch-gen/generated-live

pkill -f generated-live 2>/dev/null; sleep 1
setsid env RUST_LOG=info BIND_ADDRESS=127.0.0.1:8123 \
  DATABASE_URL="postgresql://hexgen@127.0.0.1:5433/profxdb" \
  ./target/debug/generated-live > /tmp/server.log 2>&1 < /dev/null &
SRV=$!
for i in $(seq 1 20); do
  curl -sf -o /dev/null http://127.0.0.1:8123/api-docs/openapi.json && break
  sleep 0.5
done

B=http://127.0.0.1:8123/api/v1

echo "═══ 1. OpenAPI document ═══"
curl -s http://127.0.0.1:8123/api-docs/openapi.json | python3 -c "
import json,sys; d=json.load(sys.stdin)
print('  title:', d['info']['title'], '| paths:', len(d['paths']), '| schemas:', len(d['components']['schemas']))"

echo "═══ 2. GET /users?per_page=1 (QueryBus: paginated list) ═══"
curl -s "$B/users?per_page=1" | head -c 400; echo

echo "═══ 3. POST /users (CommandBus: create) ═══"
CREATED=$(curl -s -X POST "$B/users" -H 'content-type: application/json' \
  -d '{"email":"grace@example.com","full_name":"Grace Hopper","is_active":true,"tags":["pioneer"]}')
echo "$CREATED" | head -c 400; echo
USER_ID=$(echo "$CREATED" | python3 -c "import json,sys; print(json.load(sys.stdin)['id'])")
echo "  → created id=$USER_ID"

echo "═══ 4. GET /users/$USER_ID (QueryBus: fetch one) ═══"
curl -s "$B/users/$USER_ID" | head -c 300; echo

echo "═══ 5. PUT /users/$USER_ID (CommandBus: partial update) ═══"
curl -s -X PUT "$B/users/$USER_ID" -H 'content-type: application/json' \
  -d '{"full_name":"Grace Brewster Hopper"}' | head -c 300; echo

echo "═══ 6. GET /orders/status/paid (status query) ═══"
ORDERS=$(curl -s "$B/orders/status/paid")
echo "$ORDERS" | head -c 300; echo
OID=$(echo "$ORDERS" | python3 -c "import json,sys; print(json.load(sys.stdin)[0]['id'])")
echo "  → order id=$OID"

echo "═══ 7. PATCH /orders/$OID/status (SetStatusCommand) ═══"
curl -s -X PATCH "$B/orders/$OID/status" -H 'content-type: application/json' \
  -d '{"status":"shipped"}' | python3 -c "import json,sys; print('  order_status →', json.load(sys.stdin)['order_status'])"

echo "═══ 8. DELETE /orders/$OID/soft + restore (soft delete) ═══"
curl -s -X DELETE "$B/orders/$OID/soft" -o /dev/null -w "  soft delete → HTTP %{http_code}\n"
curl -s -X POST "$B/orders/$OID/restore" | python3 -c "import json,sys; print('  restored, is_deleted =', json.load(sys.stdin)['is_deleted'])"

echo "═══ 9. GET /orders/by-user/<uid> (user-ref query) ═══"
curl -s "$B/orders/by-user/$USER_ID" | head -c 120; echo " (empty list expected — new user has no orders)"

echo "═══ 10. GET /orders/created-after?after=… (timestamp query) ═══"
curl -s "$B/orders/created-after?after=2020-01-01T00:00:00Z" | python3 -c "import json,sys; print('  orders since 2020:', len(json.load(sys.stdin)))"

echo "═══ 11. GET /product_categories (r#type keyword round-trip) ═══"
curl -s "$B/product_categories" | head -c 400; echo

echo "═══ 12. Validation: POST /users with empty email → 400 ═══"
curl -s -X POST "$B/users" -H 'content-type: application/json' -d '{"email":"  "}' \
  -o /tmp/val.json -w "  → HTTP %{http_code} "; cat /tmp/val.json; echo

echo "═══ 13. Unknown id → 404 ═══"
curl -s "$B/users/00000000-0000-0000-0000-000000000000" -o /tmp/nf.json -w "  → HTTP %{http_code} "; cat /tmp/nf.json; echo

echo "═══ 14. DELETE /users/$USER_ID (hard delete) ═══"
curl -s -X DELETE "$B/users/$USER_ID" -o /dev/null -w "  → HTTP %{http_code}\n"

kill $SRV 2>/dev/null; pkill -f generated-live 2>/dev/null
echo "═══ server log tail ═══"
tail -4 /tmp/server.log
echo "E2E COMPLETE"
