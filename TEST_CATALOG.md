# Rust Test Catalog

**Total Tests:** 115

**Numbered Tests:** 0

**Unnumbered Tests:** 115

**Numbered Tests Missing Descriptions:** 0

**Numbering Mismatches:** 115

All numbered test numbers are unique.

This catalog lists all tests in the Rust codebase.

| Test # | Function Name | Description | File |
|--------|---------------|-------------|------|
| | | | |
| unnumbered | `test_001_op_execution` | TEST001: Run Op::perform and verify the returned value matches what the op was configured with | src/op.rs:44 |
| unnumbered | `test_002_op_with_contexts` | TEST002: Verify Op reads from DryContext and produces a formatted result using that data | src/op.rs:55 |
| unnumbered | `test_003_op_default_rollback` | TEST003: Confirm that the default rollback implementation is a no-op that always succeeds | src/op.rs:89 |
| unnumbered | `test_004_op_custom_rollback` | TEST004: Verify a custom rollback implementation is called and sets the rolled_back flag | src/op.rs:114 |
| unnumbered | `test_005_perform_with_auto_logging` | TEST005: Confirm the perform() utility wraps an op with automatic logging and returns its result | src/ops.rs:97 |
| unnumbered | `test_006_caller_trigger_name` | TEST006: Verify get_caller_trigger_name() returns a string containing the module path with "::" | src/ops.rs:110 |
| unnumbered | `test_007_wrap_nested_op_exception` | TEST007: Confirm wrap_nested_op_exception wraps an error with the op name in the message | src/ops.rs:118 |
| unnumbered | `test_008_wrap_runtime_exception` | TEST008: Verify wrap_runtime_exception converts a boxed std error into an OpError::ExecutionFailed | src/ops.rs:133 |
| unnumbered | `test_009_dry_context_basic_operations` | TEST009: Insert typed values into DryContext and verify get/contains work correctly | src/contexts.rs:257 |
| unnumbered | `test_010_dry_context_builder` | TEST010: Build a DryContext with chained with_value calls and verify all values are stored | src/contexts.rs:270 |
| unnumbered | `test_011_wet_context_basic_operations` | TEST011: Insert a reference into WetContext and retrieve it by type via get_ref | src/contexts.rs:281 |
| unnumbered | `test_012_wet_context_builder` | TEST012: Build a WetContext with chained with_ref calls and verify contains for each key | src/contexts.rs:299 |
| unnumbered | `test_013_required_values` | TEST013: Confirm get_required succeeds for present keys and returns an error for missing keys | src/contexts.rs:313 |
| unnumbered | `test_014_context_merge` | TEST014: Merge two DryContexts and verify values from both are accessible in the target | src/contexts.rs:322 |
| unnumbered | `test_015_dry_context_type_mismatch_error` | TEST015: Verify get_required returns a Type mismatch error when the stored type doesn't match | src/contexts.rs:333 |
| unnumbered | `test_016_wet_context_type_mismatch_error` | TEST016: Verify WetContext get_required returns a Type mismatch error when the stored ref type differs | src/contexts.rs:364 |
| unnumbered | `test_017_control_flags` | TEST017: Set and clear abort flags on DryContext and verify is_aborted and abort_reason reflect state | src/contexts.rs:395 |
| unnumbered | `test_018_control_flags_merge` | TEST018: Merge contexts with abort flags and confirm the target inherits the abort state correctly | src/contexts.rs:417 |
| unnumbered | `test_019_get_or_insert_with` | TEST019: Verify get_or_insert_with inserts when missing and returns existing without calling factory | src/contexts.rs:444 |
| unnumbered | `test_020_get_or_compute_with` | TEST020: Verify get_or_compute_with computes and stores a value using context data and skips recompute if present | src/contexts.rs:567 |
| unnumbered | `test_021_metadata_builder` | TEST021: Build OpMetadata with name, description, and schemas and verify all fields are populated | src/op_metadata.rs:265 |
| unnumbered | `test_022_trigger_fuse` | TEST022: Construct a TriggerFuse with data and verify the trigger name and dry context values | src/op_metadata.rs:288 |
| unnumbered | `test_023_basic_validation` | TEST023: Validate a DryContext against an input schema and confirm valid/invalid reports | src/op_metadata.rs:303 |
| unnumbered | `test_024_simple_flat_outline` | TEST024: Build a flat ListingOutline with depth-0 entries and verify max_depth, levels, and flatten count | src/structured_queries.rs:303 |
| unnumbered | `test_025_hierarchical_outline` | TEST025: Build a two-level outline with chapters and sections and verify depth, level counts, and flatten | src/structured_queries.rs:319 |
| unnumbered | `test_026_complex_part_based_outline` | TEST026: Build a three-level part/chapter/section outline and verify depth and per-level entry counts | src/structured_queries.rs:341 |
| unnumbered | `test_027_flatten_preserves_hierarchy` | TEST027: Flatten a nested outline and verify each entry's path reflects its ancestry correctly | src/structured_queries.rs:374 |
| unnumbered | `test_028_schema_generation` | TEST028: Call generate_outline_schema and verify the returned JSON contains all required definitions | src/structured_queries.rs:394 |
| unnumbered | `test_029_logging_wrapper_success` | TEST029: Wrap a successful op in LoggingWrapper and verify it passes through the result unchanged | src/wrappers/logging.rs:154 |
| unnumbered | `test_030_logging_wrapper_failure` | TEST030: Wrap a failing op in LoggingWrapper and verify the error includes the op name context | src/wrappers/logging.rs:184 |
| unnumbered | `test_031_context_aware_logger` | TEST031: Use create_context_aware_logger helper and verify the wrapped op returns its result | src/wrappers/logging.rs:219 |
| unnumbered | `test_032_ansi_color_constants` | TEST032: Verify ANSI color escape code constants have the expected ANSI sequence values | src/wrappers/logging.rs:234 |
| unnumbered | `test_033_timeout_wrapper_success` | TEST033: Wrap a fast op in TimeBoundWrapper and confirm it completes before the timeout | src/wrappers/timeout.rs:174 |
| unnumbered | `test_034_timeout_wrapper_timeout` | TEST034: Wrap a slow op in TimeBoundWrapper with a short timeout and verify a Timeout error is returned | src/wrappers/timeout.rs:203 |
| unnumbered | `test_035_timeout_wrapper_with_name` | TEST035: Create a named TimeBoundWrapper and verify the op succeeds and returns the expected value | src/wrappers/timeout.rs:236 |
| unnumbered | `test_036_caller_name_wrapper` | TEST036: Use create_timeout_wrapper_with_caller_name helper and verify the op result is returned | src/wrappers/timeout.rs:268 |
| unnumbered | `test_037_logged_timeout_wrapper` | TEST037: Use create_logged_timeout_wrapper to compose logging and timeout wrappers and verify success | src/wrappers/timeout.rs:296 |
| unnumbered | `test_038_valid_input_output` | TEST038: Run ValidatingWrapper with a valid input and verify the op executes and returns the result | src/wrappers/validating.rs:231 |
| unnumbered | `test_039_invalid_input_missing_required` | TEST039: Run ValidatingWrapper without a required input field and verify a Context validation error | src/wrappers/validating.rs:245 |
| unnumbered | `test_040_invalid_input_out_of_range` | TEST040: Run ValidatingWrapper with an input exceeding the schema maximum and verify a validation error | src/wrappers/validating.rs:263 |
| unnumbered | `test_041_input_only_validation` | TEST041: Use ValidatingWrapper::input_only and confirm input is validated while output is not | src/wrappers/validating.rs:281 |
| unnumbered | `test_042_output_only_validation` | TEST042: Use ValidatingWrapper::output_only and confirm output is validated while input is not | src/wrappers/validating.rs:316 |
| unnumbered | `test_043_no_schema_validation` | TEST043: Wrap an op with no schemas in ValidatingWrapper and confirm it still succeeds | src/wrappers/validating.rs:350 |
| unnumbered | `test_044_metadata_transparency` | TEST044: Verify ValidatingWrapper::metadata() delegates to the inner op's metadata unchanged | src/wrappers/validating.rs:377 |
| unnumbered | `test_045_reference_validation` | TEST045: Verify ValidatingWrapper checks reference_schema and rejects when required refs are missing | src/wrappers/validating.rs:389 |
| unnumbered | `test_046_no_reference_schema` | TEST046: Wrap an op with no reference schema in ValidatingWrapper and confirm it succeeds | src/wrappers/validating.rs:446 |
| unnumbered | `test_047_batch_metadata_with_data_flow` | TEST047: Build BatchMetadata from producer/consumer ops and verify only external inputs are required | src/batch_metadata.rs:246 |
| unnumbered | `test_048_reference_schema_merging` | TEST048: Build BatchMetadata from two ops with different reference schemas and verify union of required refs | src/batch_metadata.rs:268 |
| unnumbered | `test_049_batch_op_success` | TEST049: Run BatchOp with two succeeding ops and verify results contain both values in order | src/batch.rs:148 |
| unnumbered | `test_050_batch_op_failure` | TEST050: Run BatchOp where the second op fails and verify the batch returns an error | src/batch.rs:164 |
| unnumbered | `test_051_batch_op_returns_all_results` | TEST051: Run BatchOp with two ops and verify both result values are present in order | src/batch.rs:180 |
| unnumbered | `test_052_batch_metadata_data_flow` | TEST052: Verify BatchOp metadata correctly identifies only the externally-required input fields | src/batch.rs:198 |
| unnumbered | `test_053_batch_reference_schema_merging` | TEST053: Verify BatchOp merges reference schemas from all ops into a unified set of required refs | src/batch.rs:287 |
| unnumbered | `test_054_batch_rollback_on_failure` | TEST054: Run BatchOp where the third op fails and verify rollback is called on the first two but not the third | src/batch.rs:360 |
| unnumbered | `test_055_batch_rollback_order` | TEST055: Run BatchOp where the last op fails and verify rollback occurs in reverse (LIFO) order | src/batch.rs:441 |
| unnumbered | `test_056_batch_rollback_on_failure_partial` | TEST056: Run BatchOp where one op fails and verify rollback is triggered for succeeded ops | src/batch.rs:511 |
| unnumbered | `test_057_abort_macro_without_reason` | TEST057: Invoke the abort macro without a reason and verify the context is aborted with no reason string | src/control_flow_tests.rs:86 |
| unnumbered | `test_058_abort_macro_with_reason` | TEST058: Invoke the abort macro with a reason string and verify abort_reason matches | src/control_flow_tests.rs:106 |
| unnumbered | `test_059_continue_loop_macro` | TEST059: Use the continue_loop macro inside an op and verify the scoped continue flag is set in context | src/control_flow_tests.rs:126 |
| unnumbered | `test_060_check_abort_macro` | TEST060: Use check_abort macro to short-circuit when the abort flag is already set in context | src/control_flow_tests.rs:147 |
| unnumbered | `test_061_batch_op_with_abort` | TEST061: Run a BatchOp where the second op aborts and verify the batch stops and propagates the abort | src/control_flow_tests.rs:171 |
| unnumbered | `test_062_batch_op_with_pre_existing_abort` | TEST062: Start a BatchOp with an abort flag already set and verify it immediately returns Aborted | src/control_flow_tests.rs:194 |
| unnumbered | `test_063_loop_op_with_continue` | TEST063: Run a LoopOp where an op signals continue and verify subsequent ops in the iteration are skipped | src/control_flow_tests.rs:219 |
| unnumbered | `test_064_loop_op_with_abort` | TEST064: Run a LoopOp where an op aborts mid-loop and verify the loop terminates with the abort error | src/control_flow_tests.rs:244 |
| unnumbered | `test_065_loop_op_with_pre_existing_abort` | TEST065: Start a LoopOp with an abort flag already set and verify it immediately returns Aborted | src/control_flow_tests.rs:267 |
| unnumbered | `test_066_complex_control_flow_scenario` | TEST066: Nest a batch with a continue op inside a loop and verify results across all iterations | src/control_flow_tests.rs:291 |
| unnumbered | `test_067_loop_op_basic` | TEST067: Run a LoopOp for 3 iterations with 2 ops each and verify all 6 results in order | src/loop_op.rs:214 |
| unnumbered | `test_068_loop_op_with_counter_access` | TEST068: Run a LoopOp where each op reads the loop counter and verify values are 0, 1, 2 | src/loop_op.rs:233 |
| unnumbered | `test_069_loop_op_existing_counter` | TEST069: Start a LoopOp with a pre-initialized counter and verify it only executes the remaining iterations | src/loop_op.rs:250 |
| unnumbered | `test_070_loop_op_zero_limit` | TEST070: Run a LoopOp with a zero iteration limit and verify no ops are executed | src/loop_op.rs:268 |
| unnumbered | `test_071_loop_op_builder_pattern` | TEST071: Build a LoopOp with add_op chaining and verify all added ops run across all iterations | src/loop_op.rs:285 |
| unnumbered | `test_072_loop_op_rollback_on_iteration_failure` | TEST072: Run a LoopOp where the third op fails and verify succeeded ops are rolled back in reverse order | src/loop_op.rs:301 |
| unnumbered | `test_073_loop_op_rollback_order_within_iteration` | TEST073: Run a LoopOp where the last op fails and verify rollback occurs in LIFO order within the iteration | src/loop_op.rs:376 |
| unnumbered | `test_074_loop_op_successful_iterations_not_rolled_back` | TEST074: Run a LoopOp that fails on iteration 2 and verify previously completed iterations are not rolled back | src/loop_op.rs:446 |
| unnumbered | `test_075_loop_op_mixed_iteration_with_rollback` | TEST075: Run a LoopOp where op2 fails on iteration 1 and verify only op1 from that iteration is rolled back | src/loop_op.rs:514 |
| unnumbered | `test_076_loop_op_continue_on_error` | TEST076: Run a LoopOp configured to continue on error and verify subsequent iterations still execute | src/loop_op.rs:683 |
| unnumbered | `test_077_dry_put_and_get` | TEST077: Use dry_put! and dry_get! macros to store and retrieve a typed value by variable name | tests/macro_tests.rs:11 |
| unnumbered | `test_078_dry_require` | TEST078: Use dry_require! macro to retrieve a required value and verify error when key is missing | tests/macro_tests.rs:23 |
| unnumbered | `test_079_dry_result` | TEST079: Use dry_result! macro to store a final result and verify it is stored under both "result" and op name | tests/macro_tests.rs:40 |
| unnumbered | `test_080_wet_put_ref_and_require_ref` | TEST080: Use wet_put_ref! and wet_require_ref! macros to store and retrieve a service reference | tests/macro_tests.rs:60 |
| unnumbered | `test_081_wet_put_arc` | TEST081: Use wet_put_arc! to store an Arc-wrapped service and retrieve it via wet_require_ref! | tests/macro_tests.rs:73 |
| unnumbered | `test_082_macros_in_op` | TEST082: Run a full op that uses dry_require! and wet_require_ref! macros internally and verify the output | tests/macro_tests.rs:109 |
| unnumbered | `test_083_error_handling_and_wrapper_chains` | TEST083: Compose timeout and logging wrappers around a failing op and verify the error message includes the op name | tests/integration_tests.rs:39 |
| unnumbered | `test_084_stack_trace_analysis` | TEST084: Call get_caller_trigger_name from within a test and verify it reflects the integration test module path | tests/integration_tests.rs:65 |
| unnumbered | `test_085_exception_wrapping_utilities` | TEST085: Use wrap_nested_op_exception and verify the wrapped error contains both op name and original message | tests/integration_tests.rs:73 |
| unnumbered | `test_086_timeout_wrapper_functionality` | TEST086: Wrap a slow op in a short-timeout TimeBoundWrapper and verify the error is wrapped with logging context | tests/integration_tests.rs:104 |
| unnumbered | `test_087_dry_and_wet_context_usage` | TEST087: Run an op that retrieves a service from WetContext and reads config values from it | tests/integration_tests.rs:162 |
| unnumbered | `test_088_batch_ops` | TEST088: Run a BatchOp with two identical user-building ops and verify both produce the expected User struct | tests/integration_tests.rs:205 |
| unnumbered | `test_089_wrapper_composition` | TEST089: Compose TimeBoundWrapper and LoggingWrapper around a simple op and verify the result passes through | tests/integration_tests.rs:230 |
| unnumbered | `test_090_perform_utility` | TEST090: Use the perform() utility function directly and verify it returns the op result with auto-logging | tests/integration_tests.rs:259 |
| unnumbered | `test_091_generate_thumbnail_macro` | TEST091: Use the op! macro to generate a thumbnail op and verify it processes the file and updates context | test_macro_fix.rs:65 |
| unnumbered | `test_092_process_file_macro` | TEST092: Use the op! macro to generate a file processing op and verify context updates and result string | test_macro_fix.rs:85 |
| unnumbered | `test_093_batch_len_and_is_empty` | TEST093: Call BatchOp::len and is_empty on empty and non-empty batches | src/batch.rs:582 |
| unnumbered | `test_094_batch_add_op` | TEST094: Use add_op to dynamically add an op and verify it is executed | src/batch.rs:596 |
| unnumbered | `test_095_batch_continue_on_error` | TEST095: Run BatchOp::with_continue_on_error and verify it collects results past failures | src/batch.rs:610 |
| unnumbered | `test_096_empty_batch_returns_empty` | TEST096: Run an empty BatchOp and verify it returns an empty result vec | src/batch.rs:626 |
| unnumbered | `test_097_nested_batch_rollback` | TEST097: Verify nested BatchOp rollback propagates correctly when outer batch fails | src/batch.rs:636 |
| unnumbered | `test_098_dry_context_merge_overwrites_keys` | TEST098: Merge two DryContexts where keys overlap and verify the merging context's values win | src/contexts.rs:488 |
| unnumbered | `test_099_wet_context_merge` | TEST099: Merge two WetContexts and verify both sets of references are accessible in the target | src/contexts.rs:500 |
| unnumbered | `test_100_dry_context_serde_roundtrip` | TEST100: Serialize and deserialize a DryContext and verify all values survive the round-trip | src/contexts.rs:517 |
| unnumbered | `test_101_dry_context_clone_is_independent` | TEST101: Clone a DryContext and verify the clone is independent (mutations don't propagate) | src/contexts.rs:533 |
| unnumbered | `test_102_dry_context_keys` | TEST102: Verify DryContext::keys() returns all inserted keys | src/contexts.rs:543 |
| unnumbered | `test_103_wet_context_keys` | TEST103: Verify WetContext::keys() returns all inserted reference keys | src/contexts.rs:555 |
| unnumbered | `test_104_op_error_display_execution_failed` | TEST104: Verify OpError::ExecutionFailed displays with the correct message format | src/error.rs:54 |
| unnumbered | `test_105_op_error_display_timeout` | TEST105: Verify OpError::Timeout displays with the correct timeout_ms value | src/error.rs:61 |
| unnumbered | `test_106_op_error_display_context` | TEST106: Verify OpError::Context displays with the correct message format | src/error.rs:68 |
| unnumbered | `test_107_op_error_display_aborted` | TEST107: Verify OpError::Aborted displays with the correct message format | src/error.rs:75 |
| unnumbered | `test_108_op_error_clone_execution_failed` | TEST108: Clone an OpError::ExecutionFailed and verify the clone is identical | src/error.rs:82 |
| unnumbered | `test_109_op_error_clone_timeout` | TEST109: Clone OpError::Timeout and verify timeout_ms is preserved | src/error.rs:94 |
| unnumbered | `test_110_op_error_clone_other_converts_to_execution_failed` | TEST110: Clone OpError::Other and verify it becomes ExecutionFailed with the error message preserved | src/error.rs:105 |
| unnumbered | `test_111_op_error_from_serde_json_error` | TEST111: Convert a serde_json::Error into OpError via From impl | src/error.rs:119 |
| unnumbered | `test_112_output_only_still_validates_references` | TEST112: Verify ValidatingWrapper::output_only validates references even when input validation is disabled | src/wrappers/validating.rs:473 |
| unnumbered | `test_113_loop_op_break_terminates_loop` | TEST113: Run a LoopOp where an op sets the break flag and verify the loop terminates early | src/loop_op.rs:610 |
| unnumbered | `test_114_loop_op_continue_on_error_skips_failed_iterations` | TEST114: Run LoopOp::with_continue_on_error where an op fails and verify the loop continues | src/loop_op.rs:628 |
| unnumbered | `test_115_loop_op_with_no_ops_produces_no_results` | TEST115: Run an empty LoopOp with a non-zero limit and verify it produces no results | src/loop_op.rs:671 |
---

## Unnumbered Tests

The following tests are cataloged but do not currently participate in numeric test indexing.

- `test_001_op_execution` — src/op.rs:44
- `test_002_op_with_contexts` — src/op.rs:55
- `test_003_op_default_rollback` — src/op.rs:89
- `test_004_op_custom_rollback` — src/op.rs:114
- `test_005_perform_with_auto_logging` — src/ops.rs:97
- `test_006_caller_trigger_name` — src/ops.rs:110
- `test_007_wrap_nested_op_exception` — src/ops.rs:118
- `test_008_wrap_runtime_exception` — src/ops.rs:133
- `test_009_dry_context_basic_operations` — src/contexts.rs:257
- `test_010_dry_context_builder` — src/contexts.rs:270
- `test_011_wet_context_basic_operations` — src/contexts.rs:281
- `test_012_wet_context_builder` — src/contexts.rs:299
- `test_013_required_values` — src/contexts.rs:313
- `test_014_context_merge` — src/contexts.rs:322
- `test_015_dry_context_type_mismatch_error` — src/contexts.rs:333
- `test_016_wet_context_type_mismatch_error` — src/contexts.rs:364
- `test_017_control_flags` — src/contexts.rs:395
- `test_018_control_flags_merge` — src/contexts.rs:417
- `test_019_get_or_insert_with` — src/contexts.rs:444
- `test_020_get_or_compute_with` — src/contexts.rs:567
- `test_021_metadata_builder` — src/op_metadata.rs:265
- `test_022_trigger_fuse` — src/op_metadata.rs:288
- `test_023_basic_validation` — src/op_metadata.rs:303
- `test_024_simple_flat_outline` — src/structured_queries.rs:303
- `test_025_hierarchical_outline` — src/structured_queries.rs:319
- `test_026_complex_part_based_outline` — src/structured_queries.rs:341
- `test_027_flatten_preserves_hierarchy` — src/structured_queries.rs:374
- `test_028_schema_generation` — src/structured_queries.rs:394
- `test_029_logging_wrapper_success` — src/wrappers/logging.rs:154
- `test_030_logging_wrapper_failure` — src/wrappers/logging.rs:184
- `test_031_context_aware_logger` — src/wrappers/logging.rs:219
- `test_032_ansi_color_constants` — src/wrappers/logging.rs:234
- `test_033_timeout_wrapper_success` — src/wrappers/timeout.rs:174
- `test_034_timeout_wrapper_timeout` — src/wrappers/timeout.rs:203
- `test_035_timeout_wrapper_with_name` — src/wrappers/timeout.rs:236
- `test_036_caller_name_wrapper` — src/wrappers/timeout.rs:268
- `test_037_logged_timeout_wrapper` — src/wrappers/timeout.rs:296
- `test_038_valid_input_output` — src/wrappers/validating.rs:231
- `test_039_invalid_input_missing_required` — src/wrappers/validating.rs:245
- `test_040_invalid_input_out_of_range` — src/wrappers/validating.rs:263
- `test_041_input_only_validation` — src/wrappers/validating.rs:281
- `test_042_output_only_validation` — src/wrappers/validating.rs:316
- `test_043_no_schema_validation` — src/wrappers/validating.rs:350
- `test_044_metadata_transparency` — src/wrappers/validating.rs:377
- `test_045_reference_validation` — src/wrappers/validating.rs:389
- `test_046_no_reference_schema` — src/wrappers/validating.rs:446
- `test_047_batch_metadata_with_data_flow` — src/batch_metadata.rs:246
- `test_048_reference_schema_merging` — src/batch_metadata.rs:268
- `test_049_batch_op_success` — src/batch.rs:148
- `test_050_batch_op_failure` — src/batch.rs:164
- `test_051_batch_op_returns_all_results` — src/batch.rs:180
- `test_052_batch_metadata_data_flow` — src/batch.rs:198
- `test_053_batch_reference_schema_merging` — src/batch.rs:287
- `test_054_batch_rollback_on_failure` — src/batch.rs:360
- `test_055_batch_rollback_order` — src/batch.rs:441
- `test_056_batch_rollback_on_failure_partial` — src/batch.rs:511
- `test_057_abort_macro_without_reason` — src/control_flow_tests.rs:86
- `test_058_abort_macro_with_reason` — src/control_flow_tests.rs:106
- `test_059_continue_loop_macro` — src/control_flow_tests.rs:126
- `test_060_check_abort_macro` — src/control_flow_tests.rs:147
- `test_061_batch_op_with_abort` — src/control_flow_tests.rs:171
- `test_062_batch_op_with_pre_existing_abort` — src/control_flow_tests.rs:194
- `test_063_loop_op_with_continue` — src/control_flow_tests.rs:219
- `test_064_loop_op_with_abort` — src/control_flow_tests.rs:244
- `test_065_loop_op_with_pre_existing_abort` — src/control_flow_tests.rs:267
- `test_066_complex_control_flow_scenario` — src/control_flow_tests.rs:291
- `test_067_loop_op_basic` — src/loop_op.rs:214
- `test_068_loop_op_with_counter_access` — src/loop_op.rs:233
- `test_069_loop_op_existing_counter` — src/loop_op.rs:250
- `test_070_loop_op_zero_limit` — src/loop_op.rs:268
- `test_071_loop_op_builder_pattern` — src/loop_op.rs:285
- `test_072_loop_op_rollback_on_iteration_failure` — src/loop_op.rs:301
- `test_073_loop_op_rollback_order_within_iteration` — src/loop_op.rs:376
- `test_074_loop_op_successful_iterations_not_rolled_back` — src/loop_op.rs:446
- `test_075_loop_op_mixed_iteration_with_rollback` — src/loop_op.rs:514
- `test_076_loop_op_continue_on_error` — src/loop_op.rs:683
- `test_077_dry_put_and_get` — tests/macro_tests.rs:11
- `test_078_dry_require` — tests/macro_tests.rs:23
- `test_079_dry_result` — tests/macro_tests.rs:40
- `test_080_wet_put_ref_and_require_ref` — tests/macro_tests.rs:60
- `test_081_wet_put_arc` — tests/macro_tests.rs:73
- `test_082_macros_in_op` — tests/macro_tests.rs:109
- `test_083_error_handling_and_wrapper_chains` — tests/integration_tests.rs:39
- `test_084_stack_trace_analysis` — tests/integration_tests.rs:65
- `test_085_exception_wrapping_utilities` — tests/integration_tests.rs:73
- `test_086_timeout_wrapper_functionality` — tests/integration_tests.rs:104
- `test_087_dry_and_wet_context_usage` — tests/integration_tests.rs:162
- `test_088_batch_ops` — tests/integration_tests.rs:205
- `test_089_wrapper_composition` — tests/integration_tests.rs:230
- `test_090_perform_utility` — tests/integration_tests.rs:259
- `test_091_generate_thumbnail_macro` — test_macro_fix.rs:65
- `test_092_process_file_macro` — test_macro_fix.rs:85
- `test_093_batch_len_and_is_empty` — src/batch.rs:582
- `test_094_batch_add_op` — src/batch.rs:596
- `test_095_batch_continue_on_error` — src/batch.rs:610
- `test_096_empty_batch_returns_empty` — src/batch.rs:626
- `test_097_nested_batch_rollback` — src/batch.rs:636
- `test_098_dry_context_merge_overwrites_keys` — src/contexts.rs:488
- `test_099_wet_context_merge` — src/contexts.rs:500
- `test_100_dry_context_serde_roundtrip` — src/contexts.rs:517
- `test_101_dry_context_clone_is_independent` — src/contexts.rs:533
- `test_102_dry_context_keys` — src/contexts.rs:543
- `test_103_wet_context_keys` — src/contexts.rs:555
- `test_104_op_error_display_execution_failed` — src/error.rs:54
- `test_105_op_error_display_timeout` — src/error.rs:61
- `test_106_op_error_display_context` — src/error.rs:68
- `test_107_op_error_display_aborted` — src/error.rs:75
- `test_108_op_error_clone_execution_failed` — src/error.rs:82
- `test_109_op_error_clone_timeout` — src/error.rs:94
- `test_110_op_error_clone_other_converts_to_execution_failed` — src/error.rs:105
- `test_111_op_error_from_serde_json_error` — src/error.rs:119
- `test_112_output_only_still_validates_references` — src/wrappers/validating.rs:473
- `test_113_loop_op_break_terminates_loop` — src/loop_op.rs:610
- `test_114_loop_op_continue_on_error_skips_failed_iterations` — src/loop_op.rs:628
- `test_115_loop_op_with_no_ops_produces_no_results` — src/loop_op.rs:671

---

## Numbering Mismatches

These tests have a numbering disagreement between the function name and the authoritative immediate TEST comment/docstring above the test. This is reported explicitly so comment sync does not silently overwrite a misnumbered test.

- `unnumbered` / `test001` / `test_001_op_execution` — src/op.rs:44
- `unnumbered` / `test002` / `test_002_op_with_contexts` — src/op.rs:55
- `unnumbered` / `test003` / `test_003_op_default_rollback` — src/op.rs:89
- `unnumbered` / `test004` / `test_004_op_custom_rollback` — src/op.rs:114
- `unnumbered` / `test005` / `test_005_perform_with_auto_logging` — src/ops.rs:97
- `unnumbered` / `test006` / `test_006_caller_trigger_name` — src/ops.rs:110
- `unnumbered` / `test007` / `test_007_wrap_nested_op_exception` — src/ops.rs:118
- `unnumbered` / `test008` / `test_008_wrap_runtime_exception` — src/ops.rs:133
- `unnumbered` / `test009` / `test_009_dry_context_basic_operations` — src/contexts.rs:257
- `unnumbered` / `test010` / `test_010_dry_context_builder` — src/contexts.rs:270
- `unnumbered` / `test011` / `test_011_wet_context_basic_operations` — src/contexts.rs:281
- `unnumbered` / `test012` / `test_012_wet_context_builder` — src/contexts.rs:299
- `unnumbered` / `test013` / `test_013_required_values` — src/contexts.rs:313
- `unnumbered` / `test014` / `test_014_context_merge` — src/contexts.rs:322
- `unnumbered` / `test015` / `test_015_dry_context_type_mismatch_error` — src/contexts.rs:333
- `unnumbered` / `test016` / `test_016_wet_context_type_mismatch_error` — src/contexts.rs:364
- `unnumbered` / `test017` / `test_017_control_flags` — src/contexts.rs:395
- `unnumbered` / `test018` / `test_018_control_flags_merge` — src/contexts.rs:417
- `unnumbered` / `test019` / `test_019_get_or_insert_with` — src/contexts.rs:444
- `unnumbered` / `test020` / `test_020_get_or_compute_with` — src/contexts.rs:567
- `unnumbered` / `test021` / `test_021_metadata_builder` — src/op_metadata.rs:265
- `unnumbered` / `test022` / `test_022_trigger_fuse` — src/op_metadata.rs:288
- `unnumbered` / `test023` / `test_023_basic_validation` — src/op_metadata.rs:303
- `unnumbered` / `test024` / `test_024_simple_flat_outline` — src/structured_queries.rs:303
- `unnumbered` / `test025` / `test_025_hierarchical_outline` — src/structured_queries.rs:319
- `unnumbered` / `test026` / `test_026_complex_part_based_outline` — src/structured_queries.rs:341
- `unnumbered` / `test027` / `test_027_flatten_preserves_hierarchy` — src/structured_queries.rs:374
- `unnumbered` / `test028` / `test_028_schema_generation` — src/structured_queries.rs:394
- `unnumbered` / `test029` / `test_029_logging_wrapper_success` — src/wrappers/logging.rs:154
- `unnumbered` / `test030` / `test_030_logging_wrapper_failure` — src/wrappers/logging.rs:184
- `unnumbered` / `test031` / `test_031_context_aware_logger` — src/wrappers/logging.rs:219
- `unnumbered` / `test032` / `test_032_ansi_color_constants` — src/wrappers/logging.rs:234
- `unnumbered` / `test033` / `test_033_timeout_wrapper_success` — src/wrappers/timeout.rs:174
- `unnumbered` / `test034` / `test_034_timeout_wrapper_timeout` — src/wrappers/timeout.rs:203
- `unnumbered` / `test035` / `test_035_timeout_wrapper_with_name` — src/wrappers/timeout.rs:236
- `unnumbered` / `test036` / `test_036_caller_name_wrapper` — src/wrappers/timeout.rs:268
- `unnumbered` / `test037` / `test_037_logged_timeout_wrapper` — src/wrappers/timeout.rs:296
- `unnumbered` / `test038` / `test_038_valid_input_output` — src/wrappers/validating.rs:231
- `unnumbered` / `test039` / `test_039_invalid_input_missing_required` — src/wrappers/validating.rs:245
- `unnumbered` / `test040` / `test_040_invalid_input_out_of_range` — src/wrappers/validating.rs:263
- `unnumbered` / `test041` / `test_041_input_only_validation` — src/wrappers/validating.rs:281
- `unnumbered` / `test042` / `test_042_output_only_validation` — src/wrappers/validating.rs:316
- `unnumbered` / `test043` / `test_043_no_schema_validation` — src/wrappers/validating.rs:350
- `unnumbered` / `test044` / `test_044_metadata_transparency` — src/wrappers/validating.rs:377
- `unnumbered` / `test045` / `test_045_reference_validation` — src/wrappers/validating.rs:389
- `unnumbered` / `test046` / `test_046_no_reference_schema` — src/wrappers/validating.rs:446
- `unnumbered` / `test047` / `test_047_batch_metadata_with_data_flow` — src/batch_metadata.rs:246
- `unnumbered` / `test048` / `test_048_reference_schema_merging` — src/batch_metadata.rs:268
- `unnumbered` / `test049` / `test_049_batch_op_success` — src/batch.rs:148
- `unnumbered` / `test050` / `test_050_batch_op_failure` — src/batch.rs:164
- `unnumbered` / `test051` / `test_051_batch_op_returns_all_results` — src/batch.rs:180
- `unnumbered` / `test052` / `test_052_batch_metadata_data_flow` — src/batch.rs:198
- `unnumbered` / `test053` / `test_053_batch_reference_schema_merging` — src/batch.rs:287
- `unnumbered` / `test054` / `test_054_batch_rollback_on_failure` — src/batch.rs:360
- `unnumbered` / `test055` / `test_055_batch_rollback_order` — src/batch.rs:441
- `unnumbered` / `test056` / `test_056_batch_rollback_on_failure_partial` — src/batch.rs:511
- `unnumbered` / `test057` / `test_057_abort_macro_without_reason` — src/control_flow_tests.rs:86
- `unnumbered` / `test058` / `test_058_abort_macro_with_reason` — src/control_flow_tests.rs:106
- `unnumbered` / `test059` / `test_059_continue_loop_macro` — src/control_flow_tests.rs:126
- `unnumbered` / `test060` / `test_060_check_abort_macro` — src/control_flow_tests.rs:147
- `unnumbered` / `test061` / `test_061_batch_op_with_abort` — src/control_flow_tests.rs:171
- `unnumbered` / `test062` / `test_062_batch_op_with_pre_existing_abort` — src/control_flow_tests.rs:194
- `unnumbered` / `test063` / `test_063_loop_op_with_continue` — src/control_flow_tests.rs:219
- `unnumbered` / `test064` / `test_064_loop_op_with_abort` — src/control_flow_tests.rs:244
- `unnumbered` / `test065` / `test_065_loop_op_with_pre_existing_abort` — src/control_flow_tests.rs:267
- `unnumbered` / `test066` / `test_066_complex_control_flow_scenario` — src/control_flow_tests.rs:291
- `unnumbered` / `test067` / `test_067_loop_op_basic` — src/loop_op.rs:214
- `unnumbered` / `test068` / `test_068_loop_op_with_counter_access` — src/loop_op.rs:233
- `unnumbered` / `test069` / `test_069_loop_op_existing_counter` — src/loop_op.rs:250
- `unnumbered` / `test070` / `test_070_loop_op_zero_limit` — src/loop_op.rs:268
- `unnumbered` / `test071` / `test_071_loop_op_builder_pattern` — src/loop_op.rs:285
- `unnumbered` / `test072` / `test_072_loop_op_rollback_on_iteration_failure` — src/loop_op.rs:301
- `unnumbered` / `test073` / `test_073_loop_op_rollback_order_within_iteration` — src/loop_op.rs:376
- `unnumbered` / `test074` / `test_074_loop_op_successful_iterations_not_rolled_back` — src/loop_op.rs:446
- `unnumbered` / `test075` / `test_075_loop_op_mixed_iteration_with_rollback` — src/loop_op.rs:514
- `unnumbered` / `test076` / `test_076_loop_op_continue_on_error` — src/loop_op.rs:683
- `unnumbered` / `test077` / `test_077_dry_put_and_get` — tests/macro_tests.rs:11
- `unnumbered` / `test078` / `test_078_dry_require` — tests/macro_tests.rs:23
- `unnumbered` / `test079` / `test_079_dry_result` — tests/macro_tests.rs:40
- `unnumbered` / `test080` / `test_080_wet_put_ref_and_require_ref` — tests/macro_tests.rs:60
- `unnumbered` / `test081` / `test_081_wet_put_arc` — tests/macro_tests.rs:73
- `unnumbered` / `test082` / `test_082_macros_in_op` — tests/macro_tests.rs:109
- `unnumbered` / `test083` / `test_083_error_handling_and_wrapper_chains` — tests/integration_tests.rs:39
- `unnumbered` / `test084` / `test_084_stack_trace_analysis` — tests/integration_tests.rs:65
- `unnumbered` / `test085` / `test_085_exception_wrapping_utilities` — tests/integration_tests.rs:73
- `unnumbered` / `test086` / `test_086_timeout_wrapper_functionality` — tests/integration_tests.rs:104
- `unnumbered` / `test087` / `test_087_dry_and_wet_context_usage` — tests/integration_tests.rs:162
- `unnumbered` / `test088` / `test_088_batch_ops` — tests/integration_tests.rs:205
- `unnumbered` / `test089` / `test_089_wrapper_composition` — tests/integration_tests.rs:230
- `unnumbered` / `test090` / `test_090_perform_utility` — tests/integration_tests.rs:259
- `unnumbered` / `test091` / `test_091_generate_thumbnail_macro` — test_macro_fix.rs:65
- `unnumbered` / `test092` / `test_092_process_file_macro` — test_macro_fix.rs:85
- `unnumbered` / `test093` / `test_093_batch_len_and_is_empty` — src/batch.rs:582
- `unnumbered` / `test094` / `test_094_batch_add_op` — src/batch.rs:596
- `unnumbered` / `test095` / `test_095_batch_continue_on_error` — src/batch.rs:610
- `unnumbered` / `test096` / `test_096_empty_batch_returns_empty` — src/batch.rs:626
- `unnumbered` / `test097` / `test_097_nested_batch_rollback` — src/batch.rs:636
- `unnumbered` / `test098` / `test_098_dry_context_merge_overwrites_keys` — src/contexts.rs:488
- `unnumbered` / `test099` / `test_099_wet_context_merge` — src/contexts.rs:500
- `unnumbered` / `test100` / `test_100_dry_context_serde_roundtrip` — src/contexts.rs:517
- `unnumbered` / `test101` / `test_101_dry_context_clone_is_independent` — src/contexts.rs:533
- `unnumbered` / `test102` / `test_102_dry_context_keys` — src/contexts.rs:543
- `unnumbered` / `test103` / `test_103_wet_context_keys` — src/contexts.rs:555
- `unnumbered` / `test104` / `test_104_op_error_display_execution_failed` — src/error.rs:54
- `unnumbered` / `test105` / `test_105_op_error_display_timeout` — src/error.rs:61
- `unnumbered` / `test106` / `test_106_op_error_display_context` — src/error.rs:68
- `unnumbered` / `test107` / `test_107_op_error_display_aborted` — src/error.rs:75
- `unnumbered` / `test108` / `test_108_op_error_clone_execution_failed` — src/error.rs:82
- `unnumbered` / `test109` / `test_109_op_error_clone_timeout` — src/error.rs:94
- `unnumbered` / `test110` / `test_110_op_error_clone_other_converts_to_execution_failed` — src/error.rs:105
- `unnumbered` / `test111` / `test_111_op_error_from_serde_json_error` — src/error.rs:119
- `unnumbered` / `test112` / `test_112_output_only_still_validates_references` — src/wrappers/validating.rs:473
- `unnumbered` / `test113` / `test_113_loop_op_break_terminates_loop` — src/loop_op.rs:610
- `unnumbered` / `test114` / `test_114_loop_op_continue_on_error_skips_failed_iterations` — src/loop_op.rs:628
- `unnumbered` / `test115` / `test_115_loop_op_with_no_ops_produces_no_results` — src/loop_op.rs:671

---

*Generated from Rust source tree*
*Total tests: 115*
*Total numbered tests: 0*
*Total unnumbered tests: 115*
*Total numbered tests missing descriptions: 0*
*Total numbering mismatches: 115*
