# Casos de uso y prompts de ejecución

Este documento deriva **casos de uso** desde [requirements.md](./requirements.md). Cada caso incluye una **historia de usuario** (formato ticket), **criterios de aceptación**, y un **prompt** autocontenido para ejecutar la validación de forma manual o pegándolo en un agente de QA.

**Fuente de verdad:** el texto en inglés de `requirements.md`. Los prompts describen comportamiento observable en producto; no reinterpretan reglas de protocolo onchain.

## Cómo usar este documento

- **QA manual:** copiar el bloque *Prompt de ejecución* del UC, anotar pass/fail y evidencias (capturas, mensajes de error exactos).
- **Agente:** el prompt incluye rol, precondiciones, pasos y resultado esperado; ajustar entorno (OS, binario, RPC de prueba) en la primera línea si aplica.
- **Trazabilidad:** al final, la tabla *Requisito (líneas) → UC* enlaza cada bloque del archivo de requisitos con uno o más casos.

---

## Plantilla vacía (copiar por cada nuevo UC)

```markdown
### UC-XXX — [Título corto]

**Historia:** Como [rol], quiero [capacidad], para [motivo/beneficio].

**Requisitos:** [requirements.md, líneas o resumen]

**Criterios de aceptación:**
- …

**Precondiciones:** …

**Prompt de ejecución (copiar):**
[Texto autocontenido: contexto, precondiciones, pasos numerados, resultado esperado, datos a registrar si falla]
```

---

## Índice por tema

| Tema | UCs |
|------|-----|
| Plataforma, builds, verificación de binario | [UC-001](#uc-001--plataforma-mínima-y-ejecución-local), [UC-002](#uc-002--builds-reproducibles-y-verificación-del-binario), [UC-003](#uc-003--instalación-y-dependencias-con-fricción-mínima) |
| Conectividad RPC / Strata | [UC-004](#uc-004--acceso-readwrite-bitcoin-y-strata-rpc-o-nodo-local), [UC-005](#uc-005--endpoint-de-confianza-o-url-personalizada), [UC-006](#uc-006--conexión-por-defecto-a-nodo-local-y-aviso-si-no-hay-nodo), [UC-007](#uc-007--nodo-strata-local-sin-configuración-extra) |
| Hardware wallet y direcciones | [UC-EJEMPLO](#uc-ejemplo--conectar-hardware-wallet-y-operar-con-una-dirección-taproot-del-account-73) |
| Multisig, nonce y navegación | [UC-008](#uc-008--elegir-multisig-según-rol-de-firmante), [UC-009](#uc-009--acceso-mediante-firma-de-nonce-y-lista-canónica), [UC-010](#uc-010--cerrar-vista-de-multisig-y-desconectar-wallet) |
| Alcance multisigs (updates) | [UC-011](#uc-011--alcance-de-multisigs-para-la-sección-de-updates) |
| Updates Approved / cancelación | [UC-012](#uc-012--updates-approved-listado-cancelación-y-broadcast) |
| Exclusiones Approved/Canceled | [UC-013](#uc-013--multisigs-sin-estados-approvedcanceled) |
| Updates Pending / aprobación / envío | [UC-014](#uc-014--updates-pending-listado-offchain-ttl-y-progreso-de-firmas), [UC-015](#uc-015--firmar-copiar-firmas-y-broadcast-de-aprobación), [UC-016](#uc-016--quorum-alcanzado-opción-de-broadcast-y-botón-send-con-fee) |
| Expiración y historial updates | [UC-017](#uc-017--expiración-a-7-días-updates-expired-y-past) |
| Proponer updates y tipos | [UC-018](#uc-018--proponer-nuevas-updates), [UC-019](#uc-019--tipos-de-propuesta-por-multisig) |
| Payout Administrator: alcance | [UC-020](#uc-020--alcance-exclusivo-payout-administrator) |
| block_payout Pending y firmas | [UC-021](#uc-021--block_payout-pending-listado-y-datos), [UC-022](#uc-022--exportar-raw-firmar-copiar-y-broadcast-con-quorum) |
| block_payout envío, expiración, Past | [UC-023](#uc-023--broadcast-opcional-send-fee-expiración-y-borrado), [UC-024](#uc-024--block_payout-past-historial) |
| Creación manual y automática block_payout | [UC-025](#uc-025--creación-manual-de-block_payout-pending), [UC-026](#uc-026--botón-block-payouts-auto-empaquetado-e-inputs-visibles), [UC-027](#uc-027--firmar-nuevo-block_payout-e-idempotencia-con-pending-sin-confirmar) |

---

## UC-EJEMPLO — Conectar hardware wallet y operar con una dirección Taproot del account 73'

**Historia:** Como firmante de un multisig Alpen/Strata, quiero conectar mi hardware wallet compatible con HWI, elegir una de las primeras 20 direcciones en la ruta `m/86'/0'/73'/0/n`, verla y copiarla en la app, y verificar la misma dirección en la pantalla del dispositivo, para asegurarme de que la clave que usaré para firmar corresponde a mi seed y a la derivación acordada.

**Requisitos:** [requirements.md](./requirements.md) líneas 13–23 (conexión HW, HWI: Taproot, message signing, display, compatibilidad SPS-65; derivación `m/86'/0'/73'/0/n` con `n` en 0..19; selección como clave de firma; UI + portapapeles; verificación en dispositivo; legibilidad del mensaje en pantalla del HW).

**Criterios de aceptación:**

- Con un dispositivo soportado, la app lista direcciones en `m/86'/0'/73'/0/n` para `n` de 0 a 19 inclusive.
- Tras elegir `n`, la UI muestra la dirección seleccionada y permite copiarla al portapapeles.
- Existe un flujo para mostrar esa dirección en el hardware wallet y confirmar coincidencia con la UI.
- Cualquier mensaje a firmar en el HW puede contrastarse con lo mostrado en la app.

**Precondiciones:** App de escritorio instalada o en modo desarrollo; HW conectado y desbloqueado; firmware/apps según soporte HWI del proyecto.

**Prompt de ejecución (copiar):**

```text
Contexto: Validar UC-EJEMPLO (HW wallet + derivación m/86'/0'/73'/0/n) en la app de escritorio Alpen Multisig.

Precondiciones:
- Sistema soportado (Debian LTS reciente, macOS o Windows actualizado).
- Hardware wallet compatible (HWI: Taproot, message signing, pantalla en dispositivo).
- Abrir la app y llegar al flujo de conexión de hardware wallet.

Pasos:
1. Conectar el hardware wallet y abrir el listado de direcciones.
2. Comprobar que solo se ofrecen índices n=0..19 para la cuenta 73' en la ruta indicada (Taproot / m/86'… según UI).
3. Seleccionar una dirección distinta de la por defecto (por ejemplo n=3) y confirmar en el dispositivo si el flujo lo pide.
4. Verificar en la UI que la dirección mostrada es coherente con el índice elegido.
5. Usar copiar al portapapeles y pegar en un editor; comparar con la UI.
6. Ejecutar mostrar en dispositivo / verificación en pantalla del HW y confirmar coincidencia con lo copiado.
7. Si hay firma de mensaje o prueba de posesión, comparar texto legible en HW y en la app.

Resultado esperado:
- Sin errores de conexión; lista acotada a 20 direcciones; copia y verificación en dispositivo exitosas; el usuario puede detectar discrepancias entre pantallas.

Si falla: anotar mensaje de error exacto, OS, modelo de HW y versión de firmware.
```

---

## Plataforma, instalación y distribución

### UC-001 — Plataforma mínima y ejecución local

**Historia:** Como operador de la aplicación, quiero ejecutar el cliente de escritorio en un SO soportado con hardware mínimo definido, para poder usar el multisig en mi entorno de trabajo.

**Requisitos:** Línea 1 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- La app arranca y es usable en Debian LTS reciente, macOS o Windows actualizado, con al menos 8 GB RAM, CPU 2c4t, 1 TB SSD y conectividad ~20 Mbps según especificación.

**Precondiciones:** Máquina dentro del perfil hardware/red; SO en versión soportada.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-001 — Ejecución local en plataforma soportada.

Precondiciones: Documentar OS exacto (nombre + versión), RAM, CPU, disco y prueba de red breve (p. ej. speedtest).

Pasos:
1. Instalar o compilar la app según documentación del repo.
2. Lanzar la aplicación y confirmar que la ventana principal carga sin crash inmediato.
3. Ejecutar un flujo mínimo (p. ej. pantalla inicial o ajustes) durante 2–3 minutos.

Resultado esperado: Estabilidad básica en el hardware/OS declarados; sin requisitos ocultos que contradigan la línea 1 de requirements.md.

Si falla: logs de crash, versión exacta del SO, hardware medido.
```

### UC-002 — Builds reproducibles y verificación del binario

**Historia:** Como usuario final o auditor, quiero builds reproducibles y poder verificar criptográficamente que el binario fue publicado y aprobado por varios empleados de Alpen Labs, para reducir confianza en un solo punto de publicación.

**Requisitos:** Líneas 2–3 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- El proceso de release/documentación permite reproducir el artefacto o explica el mecanismo oficial.
- Existe un camino documentado (firma, attestations, etc.) para verificar el binario respecto de Alpen Labs según lo implementado en el proyecto.

**Precondiciones:** Acceso a documentación de release y herramientas de verificación publicadas.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-002 — Reproducibilidad y verificación del binario.

Pasos:
1. Leer docs de release/build en el repositorio y seguir el procedimiento de build reproducible si está descrito.
2. Descargar un binario publicado oficialmente y aplicar el procedimiento de verificación criptográfica indicado por el proyecto.
3. Comprobar que la verificación involucra múltiples aprobaciones según el diseño documentado (no solo una firma anónima).

Resultado esperado: Procedimiento reproducible o justificado; verificación multi-firma o equivalente según especificación del proyecto.

Si no aplica aún en el repo: marcar N/A y enlazar issue o doc pendiente.
```

### UC-003 — Instalación y dependencias con fricción mínima

**Historia:** Como usuario, quiero instalar o ejecutar la app con un solo comando de terminal o doble clic, y como máximo un paso extra para dependencias, para incorporar el cliente sin una guía larga.

**Requisitos:** Líneas 4–7 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Instalación o ejecución en un paso principal (comando único o icono).
- Dependencias adicionales, si existen, requieren como máximo un comando o clic extra (excluyendo prompts de privilegios de administrador).

**Precondiciones:** Máquina limpia o típica de usuario final según el escenario de prueba elegido.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-003 — Fricción mínima de instalación.

Pasos:
1. En un entorno de prueba, contar comandos/clics desde cero hasta tener la app ejecutable (sin contar aprobaciones UAC/admin explícitas del SO).
2. Si hay dependencias (runtime, drivers HW, etc.), contar pasos adicionales.
3. Comparar con el límite: 1 comando o doble clic principal + máximo 1 paso extra para deps.

Resultado esperado: Cumple líneas 4–7 de requirements.md o se documenta desviación con plan de remediación.

Evidencia: lista numerada de pasos realizados.
```

---

## Conectividad Bitcoin / Strata (RPC y nodo local)

### UC-004 — Acceso read/write Bitcoin y Strata (RPC o nodo local)

**Historia:** Como usuario, quiero que la aplicación acceda en lectura/escritura a Bitcoin y Strata mediante un RPC de confianza o un nodo Strata local en el mismo escritorio, para operar sin depender de un único modelo de infraestructura.

**Requisitos:** Línea 8 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Con RPC de confianza configurado, las operaciones que requieran chain funcionan.
- Con nodo local disponible según configuración del proyecto, el acceso read/write es posible.

**Precondiciones:** RPC de prueba o nodo local según escenario.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-004 — Read/write Bitcoin y Strata vía RPC o nodo local.

Escenario A — RPC de confianza:
1. Configurar endpoint RPC válido en la app.
2. Ejecutar una acción que requiera lectura de cadena (p. ej. estado de cuenta o saldo según UI).
3. Ejecutar una acción de escritura solo si el entorno de prueba lo permite de forma segura.

Escenario B — Nodo local:
1. Levantar nodo Strata/Bitcoin local según documentación del proyecto.
2. Conectar la app al nodo local.
3. Repetir lectura y, si aplica, escritura de prueba.

Resultado esperado: Ambos modos soportados según requirements; errores claros si el endpoint es inválido.
```

### UC-005 — Endpoint de confianza o URL personalizada

**Historia:** Como usuario, quiero elegir un endpoint de confianza bajo el dominio stratabtc.org o introducir mi propia URL de RPC, para conectar a la infraestructura que corresponda a mi rol.

**Requisitos:** Línea 9 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Opción de seleccionar RPC de confianza en https://stratabtc.org (según implementación UI).
- Campo o flujo para URL RPC personalizada.
- Las operaciones usan el endpoint seleccionado tras guardar/aplicar.

**Precondiciones:** URLs de prueba válidas o mocks según entorno.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-005 — stratabtc.org vs URL custom.

Pasos:
1. Abrir configuración de conexión / RPC en la app.
2. Seleccionar el endpoint de confianza asociado a stratabtc.org si está expuesto en UI.
3. Cambiar a una URL RPC personalizada (sandbox o regtest según disponibilidad).
4. Verificar que la app intenta conectar usando la URL activa (mensaje de éxito o error de conexión explícito).

Resultado esperado: Ambas opciones accesibles y la selección persiste durante la sesión según diseño.
```

### UC-006 — Conexión por defecto a nodo local y aviso si no hay nodo

**Historia:** Como usuario, quiero que por defecto la app intente un nodo local y, si no lo detecta, que me pida encender el nodo o cambiar a RPC de confianza, para no quedarme conectado sin saberlo a un backend incorrecto.

**Requisitos:** Líneas 10–11 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- El método por defecto es nodo local en el mismo escritorio.
- Sin nodo local, aparece un prompt claro: encender nodo local o cambiar a RPC de confianza.

**Precondiciones:** Control sobre si el nodo local está encendido o apagado.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-006 — Default local + prompt sin nodo.

Pasos:
1. Con nodo local apagado y sin configuración manual previa, abrir la app o la sección de conexión.
2. Observar el comportamiento por defecto y cualquier diálogo.
3. Confirmar que se ofrece alternativa de RPC de confianza o instrucción para iniciar nodo local.
4. Encender el nodo local y verificar que la app lo detecta sin pasos ocultos contradictorios al requisito.

Resultado esperado: Cumple líneas 10–11; mensajes de alto señal.
```

### UC-007 — Nodo Strata local sin configuración extra

**Historia:** Como usuario con nodo Strata local en ejecución, quiero que la app use ese nodo para Bitcoin y Strata sin esfuerzo adicional, para reducir fricción operativa.

**Requisitos:** Línea 12 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Con nodo local estándar del ecosistema en marcha, la app se conecta o guía con mínimos pasos adicionales (SHOULD).

**Precondiciones:** Nodo Strata local instalado según doc del proyecto.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-007 — SHOULD de nodo local sin esfuerzo extra.

Precondiciones: Nodo Strata local corriendo en la misma máquina con configuración por defecto del proyecto.

Pasos:
1. Abrir la app sin haber configurado RPC remoto previamente.
2. Contar clics/comandos necesarios hasta tener conexión read/write funcional.
3. Documentar cualquier paso “extra” no trivial.

Resultado esperado: Cumple la intención del SHOULD en línea 12 o se lista brecha con severidad.
```

---

## Multisig, autenticación y navegación

### UC-008 — Elegir multisig según rol de firmante

**Historia:** Como firmante ya conectado con mi dirección, quiero elegir un multisig de una lista acotada a aquellos donde mi dirección es signer, para trabajar solo en contextos autorizados.

**Requisitos:** Líneas 24–29 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Lista incluye solo tipos soportados: Alpen Administrator, Strata Administrator, Strata Sequencer Manager, Strata Security Council, Payout Administrator.
- Cada opción es usable exclusivamente por los signers de ese rol (según lista canónica posterior).

**Precondiciones:** Dirección conectada que figure como signer en al menos un multisig de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-008 — Selección de multisig.

Pasos:
1. Conectar wallet y dirección válida para entorno de prueba.
2. Abrir la lista de multisigs disponibles para esa dirección.
3. Verificar que solo aparecen multisigs aplicables y que la nomenclatura coincide con los cinco tipos del requisito.

Resultado esperado: Lista coherente con L24–29; no aparecen multisigs no soportados.
```

### UC-009 — Acceso mediante firma de nonce y lista canónica

**Historia:** Como firmante, quiero firmar un nonce con la clave de mi dirección conectada para entrar al UI del multisig elegido, y solo obtener acceso si mi dirección está en la lista canónica de signers.

**Requisitos:** Líneas 30–31 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Tras elegir multisig, el flujo exige firma de nonce con la clave de la dirección conectada.
- Sin estar en la lista canónica del multisig seleccionado, no hay acceso al UI de ese multisig.

**Precondiciones:** Dirección en lista y dirección fuera de lista para pruebas negativas.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-009 — Nonce + lista canónica.

Pasos:
1. Con dirección en lista canónica: elegir multisig, firmar nonce en HW/software según flujo, confirmar acceso al UI.
2. Con dirección válida criptográficamente pero NO en lista del multisig elegido: intentar mismo flujo.
3. Verificar denegación de acceso al UI del multisig en el caso 2.

Resultado esperado: L30–31 cumplidos; en caso 2 debe mostrarse el error descrito en UC-009b si aplica.
```

### UC-009b — Errores: firma inválida o no signer

**Historia:** Como usuario, quiero mensajes claros cuando mi firma del nonce es inválida o cuando mi firma es válida pero mi dirección no es signer del multisig, para entender por qué no obtengo acceso.

**Requisitos:** Líneas 32–33 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Firma inválida: mensaje indicando firma inválida (SHOULD).
- Firma válida pero dirección no en lista de signers del multisig: mensaje explícito (SHOULD).

**Precondiciones:** Capacidad de simular firma incorrecta o usar dirección no autorizada.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-009b — Errores de autenticación.

Pasos:
1. Provocar firma inválida (rechazar en dispositivo o alterar payload si el entorno lo permite de forma controlada).
2. Observar mensaje de error.
3. Usar dirección válida pero no signer (L33) y observar mensaje.

Resultado esperado: Mensajes distintos y comprensibles alineados a L32–33.
```

### UC-010 — Cerrar vista de multisig y desconectar wallet

**Historia:** Como firmante autenticado, quiero volver a la pantalla de selección de multisig o desconectar mi dirección y volver a conexión de wallet, para cambiar de contexto de forma segura.

**Requisitos:** Líneas 34–35 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Acción para cerrar el UI del multisig seleccionado y regresar a selección de multisigs.
- Acción para desconectar la dirección y regresar a la pantalla de conexión de wallet.

**Precondiciones:** Sesión autenticada en un multisig.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-010 — Navegación atrás.

Pasos:
1. Estando dentro del UI de un multisig tras nonce válido, usar “cerrar” / equivalente y verificar regreso a lista de multisigs.
2. Desde el mismo estado, usar “desconectar wallet” / equivalente y verificar regreso a flujo de conexión de dirección.

Resultado esperado: L34–35; no quedan datos sensibles en pantalla que correspondan a la sesión anterior (según diseño de privacidad de la app).
```

### UC-011 — Alcance de multisigs para la sección de updates

**Historia:** Como auditor de requisitos, quiero saber qué multisigs aplica la sección de updates (Approved/Pending/etc.), para no exigir comportamientos incorrectos en multisigs fuera de alcance.

**Requisitos:** Líneas 36–40 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- La documentación de prueba reconoce que las reglas de updates bajo esa sección aplican solo a: Alpen Administrator, Strata Administrator, Strata Sequencer Manager, Strata Security Council (salvo exclusiones explícitas en subsecciones).

**Precondiciones:** Ninguna ejecución de producto obligatoria; revisión de alcance.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-011 — Alcance multisigs (updates).

Pasos:
1. Leer L36–40 y L46–48 de requirements.md.
2. Para cada multisig listado, anotar si la UI implementa vistas de updates y si coincide con el alcance declarado.

Resultado esperado: Matriz multisig × secciones de UI coherente con el documento de requisitos.
```

---

## Updates (Alpen/Strata — estados y flujos)

### UC-012 — Updates Approved: listado, cancelación y broadcast

**Historia:** Como signer de un multisig con updates Approved, quiero ver todas las Approved con el conteo de firmas de cancelación, poder cancelar, copiar firmas de cancelación y armar/broadcastear la tx de cancelación vía RPC de la app o raw en portapapeles, y que las canceladas permanezcan offchain solo visibles para signers.

**Requisitos:** Líneas 41–45 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Listado Approved con conteo de cancel signatures.
- Flujo de cancelación y copia de todas las cancellation signatures disponibles.
- Creación de tx de cancelación con quorum pegado y broadcast por app o externo.
- Canceled offchain y solo visibles a signers.

**Precondiciones:** Datos de prueba con updates en estado Approved según definición del requisito.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-012 — Approved updates y cancelación.

Pasos:
1. Abrir sección Approved en un multisig aplicable (ver UC-011 y exclusión UC-013).
2. Verificar columnas: identificación del update, firmas de cancelación recibidas (si any).
3. Iniciar cancelación de un update y seguir hasta obtener raw tx o broadcast por RPC de la app.
4. Copiar todas las cancellation signatures disponibles al portapapeles y validar formato.
5. Confirmar visibilidad solo para sesión de signer (según diseño offchain).

Resultado esperado: L41–45; sin fugas a usuarios no signers en entorno de prueba.
```

### UC-013 — Multisigs sin estados Approved/Canceled

**Historia:** Como tester, quiero confirmar que Strata Sequencer Manager y Strata Security Council no muestran tipos de update con estados Approved/Canceled, para alinear expectativas con L46–48.

**Requisitos:** Líneas 46–48 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- En esos multisigs, no existen flujos de UI para Approved/Canceled de updates que el requisito excluye.

**Precondiciones:** Acceso a esos multisigs en entorno de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-013 — Exclusión Approved/Canceled.

Pasos:
1. Autenticarse como signer de Strata Sequencer Manager multisig; revisar secciones de updates.
2. Repetir para Strata Security Council multisig.
3. Confirmar ausencia de estados Approved/Canceled para tipos excluidos por L46–48.

Resultado esperado: UI y backend coherentes con la exclusión; documentar cualquier discrepancia como defecto.
```

### UC-014 — Updates Pending: listado offchain, TTL y progreso de firmas

**Historia:** Como signer, quiero ver todos los updates Pending con tiempo restante hasta expiración y el progreso de firmas de aprobación, sabiendo que la lista es offchain y solo para signers.

**Requisitos:** Líneas 49–50 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Lista Pending con TTL y conteo de firmas de aprobación vs umbral.
- Datos offchain visibles solo a signers autenticados.

**Precondiciones:** Updates Pending de prueba en backend/coordinación.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-014 — Pending updates listado.

Pasos:
1. Autenticarse como signer en multisig aplicable.
2. Abrir lista Pending; verificar tiempo restante y ratio de firmas.
3. Intentar acceder sin ser signer (sesión distinta o rol no autorizado) y verificar ausencia de datos.

Resultado esperado: L49–50.
```

### UC-015 — Firmar, copiar firmas y broadcast de aprobación

**Historia:** Como signer, quiero producir una firma de aprobación para cualquier Pending update, copiar todas las approval signatures disponibles y poder completar la tx de aprobación con quorum y broadcast por RPC o raw externo.

**Requisitos:** Líneas 51–53 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Firma de aprobación por update Pending.
- Copiar todas las approval signatures al portapapeles.
- Ensamblar tx con quorum y broadcast vía app o portapapeles.

**Precondiciones:** Update Pending que acepte firma en el entorno de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-015 — Aprobación de Pending update.

Pasos:
1. Seleccionar un Pending update y generar approval signature con HW/software según flujo.
2. Usar “copiar todas las approval signatures” y validar contenido.
3. Crear approval transaction pegando quorum; broadcast con RPC integrado; repetir escenario copiando raw y broadcast externo simulado.

Resultado esperado: L51–53 sin ambigüedad en mensajes de error.
```

### UC-016 — Quorum alcanzado: opción de broadcast y botón Send con fee

**Historia:** Como signer cuyo voto completa el quorum, quiero elegir si creo/firmo/broadcasteo la tx de confirmación o lo declino; y si ya hay quorum pero falta confirmación onchain, quiero un botón Send con fee en pasos de 0.1 sat/vB.

**Requisitos:** Líneas 54–55 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Tras alcanzar quorum con mi firma, aparece opción de continuar con broadcast o declinar (SHOULD en L54).
- Con quorum pendiente de confirmación, botón Send permite fijar fee en incrementos de 0.1 sat/vB y enviar.

**Precondiciones:** Escenario de prueba donde se pueda llegar a quorum sin gastar fees reales (regtest/mock) si existe.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-016 — Broadcast opcional + Send + fee 0.1 sat/vB.

Pasos:
1. Llevar un Pending update hasta quorum con la última firma desde esta sesión; observar opción de broadcast o declinar (L54).
2. En estado quorum sin confirmación onchain, abrir flujo Send; ajustar fee en 0.1 sat/vB pasos y verificar que el control respeta incrementos.
3. Completar broadcast de prueba si el entorno lo permite.

Resultado esperado: L54–55.
```

### UC-017 — Expiración a 7 días, updates Expired y Past

**Historia:** Como signer, quiero que los Pending expiren a los 7 días sin aprobación, que los Expired sigan offchain solo para signers, y poder ver todos los Past (enacted, canceled, expired).

**Requisitos:** Líneas 56–58 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Expiración a 7 días desde la primera propuesta (según requisito).
- Expired offchain solo signers.
- Lista Past incluye updates finalizados por cualquiera de los tres finales.

**Precondiciones:** Control de reloj o datos simulados con timestamps de expiración.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-017 — TTL 7d, Expired, Past.

Pasos:
1. Crear o usar update Pending cuyo reloj de expiración pueda observarse (test con tiempo acelerado o fixtures).
2. Verificar transición a Expired tras ventana de 7 días según definición del requisito.
3. Verificar visibilidad de Expired solo para signers.
4. Abrir lista Past y validar que incluye enacted, canceled y expired según definición L58.

Resultado esperado: L56–58; si no hay aceleración de tiempo en entorno, marcar prueba como parcial y documentar dependencia de datos.
```

### UC-018 — Proponer nuevas updates

**Historia:** Como signer, quiero proponer nuevas updates en todos los multisigs en los que soy signer, para iniciar cambios de protocolo o administración según mis permisos.

**Requisitos:** Línea 59 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Flujo de “proponer update” disponible para cada multisig donde la dirección autenticada es signer.

**Precondiciones:** Permisos de proposición en entorno de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-018 — Proponer updates.

Pasos:
1. Por cada multisig donde la cuenta de prueba es signer, localizar acción de nueva propuesta.
2. Completar formulario mínimo válido o hasta el punto seguro previo a broadcast real.
3. Verificar que la propuesta aparece como Pending u offchain según diseño.

Resultado esperado: L59 cumplido para todos los multisigs aplicables a la cuenta de prueba.
```

### UC-019 — Tipos de propuesta por multisig

**Historia:** Como signer, quiero que el sistema soporte los tipos de update listados para cada multisig (Alpen Administrator, Strata Administrator, Strata Sequencer Manager, Security Council), para cubrir el catálogo de cambios operativos.

**Requisitos:** Líneas 60–76 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Para cada multisig del listado, existen tipos de propuesta correspondientes a las viñetas del requisito (según implementación por fases del proyecto).

**Precondiciones:** Matriz de tipos vs multisig según `requirements.md`.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-019 — Catálogo de tipos de propuesta.

Pasos:
1. Construir tabla desde L60–76 de requirements.md (multisig × tipos).
2. Por cada fila, abrir UI de “nueva propuesta” y verificar que el tipo existe o documentar “no implementado aún”.
3. Para un subconjunto representativo, crear borrador de propuesta hasta el paso previo a firma final.

Resultado esperado: Cobertura explícita de catálogo; brechas documentadas con ID de feature.
```

---

## Payout Administrator — block_payout

### UC-020 — Alcance exclusivo Payout Administrator

**Historia:** Como lector de requisitos, quiero distinguir qué reglas de block_payout aplican solo al multisig Payout Administrator, para no confundirlas con otras secciones.

**Requisitos:** Línea 77 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Las pruebas de L78–95 se ejecutan en contexto Payout Administrator salvo que el propio texto indique lo contrario.

**Precondiciones:** Ninguna.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-020 — Alcance Payout Administrator.

Pasos:
1. Leer L77 y encabezar el plan de pruebas de block_payout como exclusivo de ese multisig.
2. Verificar en UI que los flujos de block_payout no aparecen en multisigs incorrectos.

Resultado esperado: Alineación documento-producto.
```

### UC-021 — block_payout Pending: listado y datos

**Historia:** Como signer de Payout Administrator, quiero ver todos los block_payout Pending con TTL, txid y progreso de firmas, manteniendo la información offchain solo para signers.

**Requisitos:** Líneas 78–79 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Listado con tiempo restante, txid, firmas recibidas vs requeridas.
- Visibilidad restringida a signers.

**Precondiciones:** Transacciones Pending de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-021 — Listado Pending block_payout.

Pasos:
1. Autenticarse en Payout Administrator multisig.
2. Abrir sección Pending block_payout; validar columnas L78.
3. Verificar ausencia de datos para usuario no signer.

Resultado esperado: L78–79.
```

### UC-022 — Exportar raw, firmar, copiar y broadcast con quorum

**Historia:** Como signer, quiero exportar raw de un Pending block_payout, producir spend signature, copiar todas las spend signatures y broadcastear con quorum vía app o raw externo.

**Requisitos:** Líneas 80–83 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Export raw disponible.
- Firma de gasto y copia de todas las spend signatures.
- Broadcast con quorum por RPC app o clipboard.

**Precondiciones:** Pending block_payout de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-022 — Firmas y broadcast block_payout.

Pasos:
1. Exportar raw de un Pending block_payout y verificar integridad básica (no vacío, parseable por herramienta del proyecto).
2. Añadir spend signature; copiar todas las spend signatures.
3. Pegar quorum y broadcast por RPC; repetir con broadcast externo usando raw copiado.

Resultado esperado: L80–83.
```

### UC-023 — Broadcast opcional, Send con fee, expiración y borrado

**Historia:** Como signer que completa el quorum de un block_payout, quiero poder elegir broadcast o declinar; con quorum sin confirmar, usar Send con fee en 0.1 sat/vB; si expira a 7 días desde la primera firma en sistema, debe eliminarse del backend y de la UI.

**Requisitos:** Líneas 84–87 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Opción broadcast/declinar al completar quorum (SHOULD L84).
- Send con fee 0.1 sat/vB (L85).
- Expiración L86; borrado backend + UI L87.

**Precondiciones:** Datos o simulación de expiración.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-023 — Quorum, Send, expiración y borrado.

Pasos:
1. Completar quorum y verificar opción de broadcast o declinar (L84).
2. Usar Send con ajuste fino de fee 0.1 sat/vB (L85).
3. Simular o esperar expiración según L86; verificar que el backend ya no expone el registro y la UI no lo lista (L87).

Resultado esperado: L84–87.
```

### UC-024 — block_payout Past: historial

**Historia:** Como signer, quiero ver todos los block_payout Past con estado de confirmación, timestamp de bloque y txid.

**Requisitos:** Línea 88 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Lista Past con Unconfirmed/Confirmed, block timestamp, txid.

**Precondiciones:** Historial de prueba o regtest.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-024 — Past block_payout.

Pasos:
1. Abrir sección Past en Payout Administrator.
2. Validar columnas según L88 para cada fila visible.

Resultado esperado: L88.
```

### UC-025 — Creación manual de Pending block_payout

**Historia:** Como signer, quiero crear manualmente un Pending block_payout proporcionando inputs y añadiendo mi firma.

**Requisitos:** Línea 89 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Flujo manual: inputs block_payout + firma → aparece en Pending.

**Precondiciones:** Inputs de prueba válidos.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-025 — Manual create Pending block_payout.

Pasos:
1. Abrir flujo manual de creación según UI.
2. Introducir inputs requeridos; firmar con dirección conectada.
3. Verificar aparición en sección Pending.

Resultado esperado: L89.
```

### UC-026 — Botón Block payouts: auto-empaquetado e inputs visibles

**Historia:** Como signer, quiero pulsar “Block payouts” para generar automáticamente una transacción que empaqueta el máximo de inputs no gastados que caben en una transacción estándar incluyendo espacio de firmas, y ver cuántos inputs incluye.

**Requisitos:** Líneas 90–92 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Botón Block payouts crea tx con empaquetado automático según límite estándar.
- UI muestra número de inputs incluidos.

**Precondiciones:** Cola de inputs no gastados de prueba.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-026 — Block payouts automático.

Pasos:
1. Preparar estado con múltiples inputs no gastados elegibles.
2. Pulsar Block payouts; revisar conteo de inputs mostrado vs expectativa razonable de tamaño de tx.
3. Verificar que no se excede “standard transaction” según definición usada por el proyecto (documentar criterio técnico observado).

Resultado esperado: L90–92.
```

### UC-027 — Firmar nuevo block_payout e idempotencia con Pending sin confirmar

**Historia:** Como signer, quiero añadir mi firma al nuevo block_payout para moverlo a Pending; si vuelvo a pulsar Block payouts antes de confirmar el último Pending, el sistema debe regenerar la misma transacción que el Pending más reciente.

**Requisitos:** Líneas 93–95 de [requirements.md](./requirements.md).

**Criterios de aceptación:**

- Firma añade la transacción a Pending (L93).
- Si existe Pending no confirmado y se pulsa Block payouts de nuevo, el resultado es idéntico al Pending más reciente (L94–95).

**Precondiciones:** Pending sin confirmación onchain.

**Prompt de ejecución (copiar):**

```text
Contexto: UC-027 — Firma e idempotencia Block payouts.

Pasos:
1. Generar nuevo block_payout vía Block payouts; firmar; verificar entrada en Pending.
2. Sin confirmar onchain ese Pending, pulsar Block payouts otra vez.
3. Comparar raw o identificador determinístico del tx propuesto con el Pending anterior; deben ser el mismo según L94–95.

Resultado esperado: L93–95; documentar método de comparación (hex hash, serialized tx, etc.).
```

---

## Trazabilidad: requisito (líneas en requirements.md) → UC

| Líneas (approx.) | UC(s) |
|------------------|--------|
| 1 | UC-001 |
| 2–3 | UC-002 |
| 4–7 | UC-003 |
| 8 | UC-004 |
| 9 | UC-005 |
| 10–11 | UC-006 |
| 12 | UC-007 |
| 13–23 | UC-EJEMPLO |
| 24–29 | UC-008 |
| 30–31 | UC-009 |
| 32–33 | UC-009b |
| 34–35 | UC-010 |
| 36–40 | UC-011 |
| 41–45 | UC-012 |
| 46–48 | UC-013 |
| 49–50 | UC-014 |
| 51–53 | UC-015 |
| 54–55 | UC-016 |
| 56–58 | UC-017 |
| 59 | UC-018 |
| 60–76 | UC-019 |
| 77 | UC-020 |
| 78–79 | UC-021 |
| 80–83 | UC-022 |
| 84–87 | UC-023 |
| 88 | UC-024 |
| 89 | UC-025 |
| 90–92 | UC-026 |
| 93–95 | UC-027 |
