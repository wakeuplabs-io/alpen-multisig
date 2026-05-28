# Block Payouts — Domain Overview

## Qué es esta sección

La sección **Payout Administrator** gestiona `block_payout` transactions: transacciones Bitcoin que bloquean reclamos de reembolso fraudulentos de operadores del bridge.

### Contexto: cómo funciona el bridge

Cuando un usuario quiere retirar BTC de Alpen a Bitcoin L1, un **operador** del bridge le adelanta el dinero de su propio bolsillo. Luego, el operador crea una "claim transaction" para recuperar ese dinero de los fondos bloqueados del bridge — de forma optimista: si nadie la desafía en el período de challenge, el operador cobra.

Un operador fraudulento podría intentar cobrar sin haber adelantado realmente los fondos. Si un **challenger** detecta esto, genera un **false claim report** con prueba criptográfica de que la claim es inválida.

### Rol del Payout Administrator

El Payout Administrator usa esos reports para crear una `block_payout` transaction que **gasta los UTXOs reclamados antes de que el operador fraudulento pueda hacerlo**, bloqueando el reembolso indebido.

**Flujo completo:**

1. Un challenger detecta una claim fraudulenta y genera un **false claim report**
2. Un signer del Payout Admin crea una `block_payout` tx usando los outpoints del report como inputs
3. Los demás signers la **firman** hasta alcanzar quorum
4. Una vez alcanzado el quorum, la tx se **broadcast a Bitcoin** — el operador fraudulento pierde su claim

---

## Estado actual del código

Lo que existe hoy es un **mock 100% frontend** — sin llamadas reales a backend ni Tauri IPC.

```
domain/block-payouts/
├── components/                  ← UI completa (dashboard, modales, cards)
├── hooks/use-block-payouts.ts   ← estado React, acciones mock
└── model/
    ├── block-payouts.types.ts   ← tipos definidos
    └── block-payouts.mock.ts    ← datos hardcodeados
```

Todo el estado vive en React, inicializado con datos falsos. Ninguna acción (firmar, crear tx, rebroadcast) llama a ningún servicio real. El objetivo es validar el flujo visual antes de conectarlo al backend real.

### Ejemplo de un false claim report

El usuario pega (o sube) uno o más reportes en formato JSON. Cada reporte representa un intento de retiro fraudulento detectado off-chain:

```json
{"claimId":"claim-test-001","outpoint":"aabb1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab:2","amount":500000,"proof":"a1b2c3d4e5f6a1b2c3d4e5f6"}
```

| Campo | Descripción |
|---|---|
| `claimId` | Identificador único del reclamo del operador que se está disputando |
| `outpoint` | UTXO a gastar (`txid:vout`) — el output del bridge que el operador quiere cobrar fraudulentamente |
| `amount` | Monto en satoshis de ese output |
| `proof` | Prueba criptográfica de que el reclamo del operador es falso (validación real pendiente) |

El modal parsea estos reportes, filtra outpoints ya gastados, y arma la lista de inputs de la `block_payout` transaction. En el mock, cualquier JSON con `proof` no vacío se considera válido.

---

## ¿Se necesita integrar ASM?

**No directamente.** El Payout Administrator es distinto al resto de los roles:

| Aspecto | Strata / Alpen Admin | Payout Admin |
|---|---|---|
| Signer set | Definido en **ASM state** (Strata chain) | Definido en el **Bridge multisig script** (Bitcoin L1) |
| Autenticación | Firma nonce; backend verifica contra ASM | **No usa ASM** — usa derivación BIP-86 `m/86'/0'/73'/0/0` |
| Transacciones | `MultisigAction` con envelope OP_RETURN (SSZ) | `block_payout` tx — Bitcoin puro, sin envelope ASM |

---

## Qué viene después (fuera del alcance del mock)

Cuando se integre el backend real, los puntos a conectar son:

1. **Tauri IPC** — derivar la P2TR address del hardware wallet (BIP-86) para autenticar al signer
2. **Orchestrator backend** — persistir txs pendientes, recopilar firmas entre signers, hacer broadcast
3. **Validación real de firmas** — Schnorr/Taproot en Rust (actualmente cualquier string ≥ 64 chars pasa)
4. **False claim proof validation** — validación criptográfica real (actualmente: cualquier JSON con campo `proof` no vacío pasa)

Referencias:
- PRD source: [docs/0-prd/03-prd-update.md](../0-prd/03-prd-update.md) §6
- Spec del mock UI: [docs/specs/block-payouts-ui-mock.md](../specs/block-payouts-ui-mock.md)
- Diferencia ASM vs Bitcoin L1: [docs/2-discovery/10-asm-bitcoin-state-model.md](./10-asm-bitcoin-state-model.md)
