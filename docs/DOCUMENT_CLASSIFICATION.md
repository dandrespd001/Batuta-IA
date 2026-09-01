# Clasificación documental y paridad

Cada fila usa exactamente ruta, clase, autoridad sucesora, paridad, evidencia de paridad y
mutabilidad. `null` significa que no existe una supersesión aplicable. Las clases permitidas son
`normative`, `evidence`, `decision`, `guide` y `archive`; las mutabilidades son `living`,
`append_only` e `immutable`.

## Inventario por ruta

| path | class | successor_authority | parity_verified | parity_evidence | mutability |
|---|---|---|---:|---|---|
| `.specify/memory/constitution.md` | `normative` | `null` | `false` | `null` | `living` |
| `AGENTS.md` | `guide` | `null` | `false` | `null` | `living` |
| `CONTRIBUTING.md` | `guide` | `null` | `false` | `null` | `living` |
| `README.md` | `guide` | `null` | `false` | `null` | `living` |
| `ROADMAP.md` | `guide` | `null` | `false` | `null` | `living` |
| `docs/CONTRATOS_OPERATIVOS_V2.md` | `archive` | `specs/system/execution.md` | `true` | `docs/DOCUMENT_CLASSIFICATION.md` | `immutable` |
| `docs/DEUDA_MODULAR_RUST.md` | `evidence` | `null` | `false` | `null` | `append_only` |
| `docs/DOCUMENT_CLASSIFICATION.md` | `guide` | `null` | `false` | `null` | `living` |
| `docs/ESQUEMA_CALIDAD_ROUTING.md` | `archive` | `specs/system/quality-research.md` | `true` | `docs/DOCUMENT_CLASSIFICATION.md` | `immutable` |
| `docs/ESQUEMA_MANIFIESTO.md` | `archive` | `specs/system/manifests.md` | `true` | `docs/DOCUMENT_CLASSIFICATION.md` | `immutable` |
| `docs/FASE3_EJECUCION.md` | `archive` | `specs/system/execution.md` | `true` | `docs/DOCUMENT_CLASSIFICATION.md` | `immutable` |
| `docs/FASE4_POLITICA.md` | `archive` | `specs/system/quality-research.md` | `true` | `docs/DOCUMENT_CLASSIFICATION.md` | `immutable` |
| `docs/FASE5_PANEL.md` | `archive` | `specs/system/surfaces.md` | `true` | `docs/DOCUMENT_CLASSIFICATION.md` | `immutable` |
| `docs/IMPLEMENTACION_ROUTING_V2.md` | `guide` | `ROADMAP.md` | `false` | `null` | `living` |
| `docs/REGLAS_INGENIERIA_RUST.md` | `normative` | `null` | `false` | `null` | `living` |
| `docs/TRAZABILIDAD_ROUTING_V2.md` | `evidence` | `specs/anchors.json` | `false` | `null` | `immutable` |
| `docs/adr/ADR-0001-spec-anchored-governance.md` | `decision` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/baseline.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/specs/26e21c8e651c17bf20758f160570c6ec11e31e3acc6d01b3d574836693e2e5e2.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/specs/59ddd6234aee1a95fc7db4ecfaeee0ced3befe140190f305f21e00b0f42139f7.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/specs/8d1e228e24c449136102608028b2b37403c4624529712d4e00ceba2979999042.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/specs/9274306a9ad4a83ad9e061e4617d7e547e62f841ebd8c5373a554b17e812a70a.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/specs/b4ef1a975e590d84f6b29b5787139f1aea3cd7c2d82190c774e6e567c9d42872.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/tdd-v2.jsonl` | `evidence` | `null` | `false` | `null` | `append_only` |
| `docs/evidence/tdd.jsonl` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/tdd.schema.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/v1-baseline.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/evidence/v1.sha256` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/examples/execution-grant-draft-v1.json` | `guide` | `docs/schemas/execution-grant-draft-v1.schema.json` | `false` | `null` | `immutable` |
| `docs/examples/execution-profile-v1.json` | `guide` | `docs/schemas/execution-profile-v1.schema.json` | `false` | `null` | `immutable` |
| `docs/examples/route-request-v2-invalid-client-candidate.json` | `guide` | `specs/system/state-policy-routing.md` | `false` | `null` | `immutable` |
| `docs/examples/route-request-v2.json` | `guide` | `specs/system/state-policy-routing.md` | `false` | `null` | `immutable` |
| `docs/examples/run-request-v2.json` | `guide` | `docs/schemas/run-request-v2.schema.json` | `false` | `null` | `immutable` |
| `docs/medidas/CANARIOS.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/COSTE.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/DELEGACION_MANIFEST.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/DSH_HEADLESS.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/recibos/abacus-sin-user-rojo.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/recibos/abacus-verde.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/recibos/dsh-discrepante-rojo.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/recibos/dsh-discrepante-sin-adaptador-rojo.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/medidas/recibos/dsh-verde.json` | `evidence` | `null` | `false` | `null` | `immutable` |
| `docs/schemas/execution-grant-draft-v1.schema.json` | `normative` | `null` | `false` | `null` | `immutable` |
| `docs/schemas/execution-profile-v1.schema.json` | `normative` | `null` | `false` | `null` | `immutable` |
| `docs/schemas/run-receipt-v2.schema.json` | `normative` | `null` | `false` | `null` | `immutable` |
| `docs/schemas/run-request-v2.schema.json` | `normative` | `null` | `false` | `null` | `immutable` |
| `specs/001-adopt-spec-anchoring/checklists/acceptance-evidence.md` | `evidence` | `null` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/checklists/acceptance.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `specs/001-adopt-spec-anchoring/checklists/requirements.md` | `evidence` | `null` | `false` | `null` | `immutable` |
| `specs/001-adopt-spec-anchoring/contracts/README.md` | `decision` | `specs/schemas/spec-anchor-registry-v1.schema.json` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/data-model.md` | `decision` | `specs/schemas/spec-anchor-registry-v1.schema.json` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/feature-impact.json` | `decision` | `null` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/plan.md` | `guide` | `null` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/quickstart.md` | `guide` | `null` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/research.md` | `decision` | `null` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/spec.md` | `decision` | `specs/system/product.md` | `false` | `null` | `living` |
| `specs/001-adopt-spec-anchoring/tasks.md` | `guide` | `null` | `false` | `null` | `living` |
| `specs/README.md` | `guide` | `null` | `false` | `null` | `living` |
| `specs/anchors.json` | `guide` | `null` | `false` | `null` | `living` |
| `specs/schemas/evidence-record-v2.schema.json` | `normative` | `null` | `false` | `null` | `living` |
| `specs/schemas/feature-impact-v1.schema.json` | `normative` | `null` | `false` | `null` | `living` |
| `specs/schemas/spec-anchor-registry-v1.schema.json` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/execution.md` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/manifests.md` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/product.md` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/quality-research.md` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/rollout.md` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/state-policy-routing.md` | `normative` | `null` | `false` | `null` | `living` |
| `specs/system/surfaces.md` | `normative` | `null` | `false` | `null` | `living` |

## Procedencia de la evidencia K4

La adopción reconstruyó el inventario inicial desde el baseline K4 sellado, por lo que esa actividad
se clasifica como `reconstructed_audit`: prueba el estado observado después de la implementación y no
una secuencia red-green contemporánea. Esta clasificación no altera las etiquetas internas ni los
bytes de los 19 registros V1.

`docs/evidence/v1-baseline.json` enumera los seis artefactos V1 en `artifacts` y, por separado, el
registro histórico de corrida en `run_records`; las siete rutas son inmutables. La evidencia nueva se
añade únicamente a `docs/evidence/tdd-v2.jsonl`, cuyos registros declaran de forma individual `tdd` o
`reconstructed_audit` y enlazan un snapshot inmutable direccionado por contenido.

## Matrices de paridad de autoridades heredadas

Una fila se considera completa sólo si nombra la sección histórica, su destino vivo, al menos un
requisito y una verificación existente. El resultado `sí` significa paridad de obligación y estado;
no convierte un requisito parcial o externo en implementado.

### `docs/ESQUEMA_MANIFIESTO.md`

| Sección histórica | Autoridad viva | Requisito | Verificación | Paridad |
|---|---|---|---|---:|
| §1 Ficheros de corrida | `specs/system/manifests.md` | `REQ-MANIFESTS-001` | `crates/batuta-manifest/tests/carga.rs#declarar_lista_y_mapa_a_la_vez_no_se_admite` | sí |
| Sustituciones derivadas | `specs/system/manifests.md` | `REQ-MANIFESTS-001` | `crates/batuta-manifest/tests/carga.rs#un_mapa_de_sustitucion_incompleto_nombra_la_variante_que_falta` | sí |
| §2 Esquema completo | `specs/system/manifests.md` | `REQ-MANIFESTS-001` | `crates/batuta-manifest/tests/carga.rs#los_dos_manifiestos_del_repositorio_cargan` | sí |
| Campos nuevos | `specs/system/manifests.md` | `REQ-MANIFESTS-001`, `REQ-MANIFESTS-002` | `crates/batuta-manifest/tests/carga.rs#la_procedencia_distingue_lo_observado_de_lo_prometido` | sí |
| §3 Procedencia | `specs/system/manifests.md` | `REQ-MANIFESTS-002` | `crates/batuta-manifest/tests/carga.rs#la_procedencia_distingue_lo_observado_de_lo_prometido` | sí |
| §4 Nombres ajenos y catálogo | `specs/system/manifests.md` | `REQ-MANIFESTS-002` | `crates/batuta-manifest/tests/carga.rs#dsh_declara_canarios_reales_para_cada_capacidad_operativa` | sí |
| La bandera no es autoridad | `specs/system/manifests.md` | `REQ-MANIFESTS-002` | `crates/batuta-manifest/tests/carga.rs#la_procedencia_distingue_lo_observado_de_lo_prometido` | sí |
| Contención | `specs/system/manifests.md` | `REQ-MANIFESTS-001` | `crates/batuta-exec/tests/ejecucion.rs#al_hijo_solo_le_llega_lo_permitido` | sí |
| §4 bis Herramientas observadas | `specs/system/rollout.md` | `REQ-ROLLOUT-004` | `crates/batuta-routing/tests/operational_v2.rs#canario_tools_exige_evento_exitoso_no_una_mencion` | sí |
| §5 Límites del schema | `specs/system/manifests.md` | `REQ-MANIFESTS-001`, `REQ-MANIFESTS-002` | `crates/batuta-manifest/tests/sha256_bordes.rs#un_hash_que_no_cuadra_se_rechaza` | sí |
| §6 Aceptación Fase 2 | `specs/system/manifests.md` | `REQ-MANIFESTS-001`, `REQ-MANIFESTS-002` | `crates/batuta-manifest/tests/carga.rs#un_campo_desconocido_falla_al_cargar_y_lo_nombra` | sí |
| §7 Integración DSH | `specs/system/execution.md` | `REQ-EXECUTION-001`, `REQ-EXECUTION-003` | `crates/batuta-cli/tests/operational_api.rs#run_y_status_comparten_sobre_y_estado_durable_con_un_fake` | sí |
| Visibilidad en la interfaz | `specs/system/execution.md` | `REQ-EXECUTION-003` | `crates/batuta-routing/tests/run_receipt_v2.rs#reinicio_conserva_bytes_y_un_id_no_se_sobrescribe` | sí |
| Límite de un solo chat | `specs/system/execution.md` | `REQ-EXECUTION-002` | `crates/batuta-routing/tests/coordinator_acceptance.rs#cada_fallo_conocido_sin_retry_valido_releva_sin_historial` | sí |
| Worktree como identidad | `specs/system/execution.md` | `REQ-EXECUTION-001` | `crates/batuta-cli/tests/operational_api.rs#run_y_status_comparten_sobre_y_estado_durable_con_un_fake` | sí |
| Título identificador | `specs/system/execution.md` | `REQ-EXECUTION-003` | `crates/batuta-routing/tests/run_receipt_v2.rs#reinicio_conserva_bytes_y_un_id_no_se_sobrescribe` | sí |
| Recibo como índice | `specs/system/execution.md` | `REQ-EXECUTION-003` | `crates/batuta-routing/tests/run_receipt_v2.rs#reinicio_conserva_bytes_y_un_id_no_se_sobrescribe` | sí |
| Sin campos nuevos de manifest | `specs/system/manifests.md` | `REQ-MANIFESTS-001` | `crates/batuta-manifest/tests/carga.rs#un_campo_desconocido_falla_al_cargar_y_lo_nombra` | sí |

### `docs/CONTRATOS_OPERATIVOS_V2.md`

| Sección histórica | Autoridad viva | Requisito | Verificación | Paridad |
|---|---|---|---|---:|
| ExecutionProfileV1 | `specs/system/state-policy-routing.md` | `REQ-POLICY-001`, `REQ-POLICY-002` | `crates/batuta-cli/tests/operational_api.rs#perfil_solo_se_activa_despues_de_staging_cas_y_confirmacion` | sí |
| Política mínima K4 | `specs/system/state-policy-routing.md` | `REQ-POLICY-001` | `crates/batuta-routing/tests/execution_policy.rs#politica_es_cerrada_explicita_y_valida_limites` | sí |
| ExecutionGrantV1 | `specs/system/execution.md` | `REQ-EXECUTION-001` | `crates/batuta-routing/tests/grants.rs#grant_es_cerrado_sellado_y_nunca_admite_limites_cero` | sí |
| Ledger y reserva | `specs/system/execution.md` | `REQ-EXECUTION-001` | `crates/batuta-routing/tests/grants.rs#resultado_ambiguo_conserva_la_reserva_completa` | sí |
| RunRequest, journal y recuperación | `specs/system/execution.md` | `REQ-EXECUTION-001` | `crates/batuta-cli/tests/operational_api.rs#run_y_status_comparten_sobre_y_estado_durable_con_un_fake` | sí |
| HarnessExecutor | `specs/system/execution.md` | `REQ-EXECUTION-001`, `REQ-MANIFESTS-002` | `crates/batuta-exec/tests/manifest_executor.rs#resuelve_ruta_version_hash_argv_entorno_y_procedencia_desde_manifest` | sí |
| Salud, retry y relevo | `specs/system/execution.md` | `REQ-EXECUTION-002` | `crates/batuta-routing/tests/coordinator_acceptance.rs#retry_after_fuera_de_politica_hace_fallback_sin_dormir` | sí |
| RunReceiptV2 | `specs/system/execution.md` | `REQ-EXECUTION-003` | `crates/batuta-routing/tests/run_receipt_v2.rs#sello_alterado_o_estado_no_terminal_se_rechazan` | sí |
| Superficies K4 | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-cli/tests/tui_execution.rs#cli_formulario_tui_y_json_producen_request_decision_y_recibo_normalizados_iguales` | sí |
| Estado, ensamblado y decisión | `specs/system/state-policy-routing.md` | `REQ-STATE-001`, `REQ-ROUTING-002` | `crates/batuta-routing/tests/sealed_decision.rs#decision_sella_manifest_componentes_y_recibos_en_orden` | sí |
| Sidecar DSH | `specs/system/manifests.md` | `REQ-MANIFESTS-002` | `crates/batuta-routing/tests/dsh_sidecar.rs#usa_catalogo_sin_stream_y_opencode_desconocido_no_llega_al_selector` | sí |
| ResearchProposalV2 | `specs/system/quality-research.md` | `REQ-RESEARCH-001` | `crates/batuta-quality/tests/research.rs#q6_stage_no_modifica_evidencia_activa_y_apply_exige_confirmacion` | sí |
| CapabilityCanaryReceiptV2 | `specs/system/rollout.md` | `REQ-ROLLOUT-004` | `crates/batuta-routing/tests/operational_v2.rs#canarios_read_write_y_web_exigen_efectos_exactos` | sí |

### `docs/ESQUEMA_CALIDAD_ROUTING.md`

| Sección histórica | Autoridad viva | Requisito | Verificación | Paridad |
|---|---|---|---|---:|
| Identificadores | `specs/system/product.md` | `REQ-CONTRACTS-001` | `crates/batuta-contract/tests/identificadores.rs#route_model_es_opaco_pero_no_cualquier_cosa` | sí |
| Versiones | `specs/system/product.md` | `REQ-CONTRACTS-001` | `crates/batuta-contract/tests/identificadores.rs#la_version_de_esquema_en_curso_es_uno` | sí |
| Fechas y puntajes | `specs/system/quality-research.md` | `REQ-QUALITY-001` | `crates/batuta-quality/tests/calidad.rs#pesos_que_no_suman_cien_se_rechazan` | sí |
| Compatibilidad de cesta | `specs/system/quality-research.md` | `REQ-QUALITY-001`, `REQ-QUALITY-002` | `crates/batuta-quality/tests/calidad.rs#una_ruta_sin_revision_no_mezcla_dos_revisiones` | sí |
| Frontera pública de routing | `specs/system/state-policy-routing.md` | `REQ-ROUTING-001`, `REQ-ROUTING-002` | `crates/batuta-routing/tests/request_profile.rs#un_valor_presente_gana_al_perfil_y_una_accion_distinta_falla` | sí |
| Estado v2 | `specs/system/state-policy-routing.md` | `REQ-STATE-001` | `crates/batuta-routing/tests/state_store.rs#un_fallo_escribiendo_objetos_conserva_el_manifest_anterior` | sí |
| Compatibilidad y migración | `specs/system/state-policy-routing.md` | `REQ-STATE-002`, `REQ-POLICY-002` | `crates/batuta-routing/tests/policy_migration.rs#dry_run_no_escribe_y_apply_conserva_v1_recuperable` | sí |
| Hashes | `specs/system/state-policy-routing.md` | `REQ-ROUTING-002` | `crates/batuta-quality/tests/calidad.rs#hash_y_resultado_no_dependen_del_orden_de_entrada` | sí |
| Errores mínimos | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-routing/tests/public_discards.rs#cada_descarte_publico_expone_codigo_campo_mensaje_y_detalles` | sí |
| Evidencia de implementación | `specs/README.md` | `REQ-SDD-007` | `scripts_ci/validate_tdd_evidence.py#main` | sí |

### `docs/FASE3_EJECUCION.md`

| Sección histórica | Autoridad viva | Requisito | Verificación | Paridad |
|---|---|---|---|---:|
| 1 Máquina de estados | `specs/system/execution.md` | `REQ-EXECUTION-001` | `crates/batuta-routing/tests/run_state.rs#una_ruta_en_ejecucion_no_admite_arrancar_otra_en_paralelo` | sí |
| 2 Salud durable | `specs/system/execution.md` | `REQ-EXECUTION-002` | `crates/batuta-routing/tests/health_store.rs#salud_durable_conserva_cooldown_y_bloqueo_de_harness` | sí |
| 3 HandoffCheckpoint | `specs/system/execution.md` | `REQ-EXECUTION-002` | `crates/batuta-routing/tests/health_handoff.rs#checkpoint_valida_objetivo_fallo_siguiente_paso_y_rutas_relativas` | sí |
| 4 Recibo de routing | `specs/system/execution.md` | `REQ-EXECUTION-003` | `crates/batuta-routing/tests/routing_receipt.rs#reiniciar_conserva_el_recibo_y_un_id_no_se_sobrescribe` | sí |
| 5 Pruebas y canarios | `specs/system/rollout.md` | `REQ-ROLLOUT-004` | `crates/batuta-routing/tests/operational_v2.rs#canarios_read_write_y_web_exigen_efectos_exactos` | sí |
| 6 Aceptación | `specs/system/execution.md` | `REQ-EXECUTION-001`, `REQ-EXECUTION-002`, `REQ-EXECUTION-003` | `crates/batuta-routing/tests/coordinator_acceptance.rs#cada_fallo_conocido_sin_retry_valido_releva_sin_historial` | sí |

### `docs/FASE4_POLITICA.md`

| Sección histórica | Autoridad viva | Requisito | Verificación | Paridad |
|---|---|---|---|---:|
| 1 Vocabulario y límites | `specs/system/state-policy-routing.md` | `REQ-POLICY-001` | `crates/batuta-routing/tests/policy.rs#cada_alias_resuelve_una_ruta_exacta` | sí |
| 2 Esquema de evidencia | `specs/system/quality-research.md` | `REQ-QUALITY-001`, `REQ-QUALITY-002` | `crates/batuta-quality/tests/calidad.rs#q5_el_override_conserva_el_valor_investigado_y_no_inventa_verificacion` | sí |
| 3 Proyección y confianza | `specs/system/quality-research.md` | `REQ-QUALITY-001` | `crates/batuta-quality/tests/calidad.rs#q3_cobertura_rango_y_caducidad_son_visibles` | sí |
| 4 Política y selección | `specs/system/state-policy-routing.md` | `REQ-POLICY-001`, `REQ-ROUTING-001` | `crates/batuta-routing/tests/selector.rs#enumera_capacidad_cooldown_y_desempata_por_identificador` | sí |
| 5 Investigación bajo demanda | `specs/system/quality-research.md` | `REQ-RESEARCH-001`, `REQ-RESEARCH-002` | `specs/system/quality-research.md#REQ-RESEARCH-002` | sí |
| 6 Ejemplo de petición y decisión | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-cli/tests/routing_surfaces.rs#los_ejemplos_json_del_spec_se_validan_automaticamente` | sí |
| 7 Aceptación | `specs/system/quality-research.md` | `REQ-QUALITY-001`, `REQ-QUALITY-002`, `REQ-RESEARCH-001` | `crates/batuta-quality/tests/research.rs#una_propuesta_alterada_no_se_aplica` | sí |

### `docs/FASE5_PANEL.md`

| Sección histórica | Autoridad viva | Requisito | Verificación | Paridad |
|---|---|---|---|---:|
| 1 Una aplicación, tres superficies | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-cli/tests/routing_surfaces.rs#cli_json_y_mcp_devuelven_exactamente_la_misma_decision` | sí |
| 2 Operaciones | `specs/system/surfaces.md` | `REQ-SURFACES-002` | `specs/system/surfaces.md#REQ-SURFACES-002` | sí |
| 3 CLI JSON | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-cli/tests/routing_surfaces.rs#cli_json_resuelve_umbrales_omitidos_desde_el_perfil` | sí |
| 4 MCP por stdio | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-cli/tests/routing_surfaces.rs#mcp_no_expone_aceptar_aplicar_parches` | sí |
| 5 TUI | `specs/system/surfaces.md` | `REQ-SURFACES-001`, `REQ-SURFACES-002` | `crates/batuta-cli/tests/tui_execution.rs#controles_interactivos_admiten_grant_run_formulario_json_preview_y_ejecucion` | sí |
| 6 Invariantes de interfaz | `specs/system/surfaces.md` | `REQ-SURFACES-001` | `crates/batuta-cli/tests/routing_surfaces.rs#tabla_html_y_tui_muestran_los_mismos_campos_de_la_decision` | sí |
| 7 Aceptación | `specs/system/surfaces.md` | `REQ-SURFACES-001`, `REQ-SURFACES-002` | `crates/batuta-cli/tests/tui_execution.rs#perfil_tui_hace_staging_y_exige_escribir_el_id_para_aplicar` | sí |

## Resultado de supersesión

Las seis matrices cubren todas sus secciones normativas y ninguna fila carece de requisito o
verificación. Por ello esas seis rutas pasan a `archive`; el único cambio permitido sobre su historia
en T016 es el aviso inicial que enlaza esta evidencia y las specs sucesoras. Si una matriz perdiera
un destino o una verificación, la transición dejaría de ser válida y el documento anterior debería
recuperar clase `normative` hasta reparar la paridad.
