# Rust Test Catalog

**Total Tests:** 120

**Numbered Tests:** 120

**Unnumbered Tests:** 0

**Numbered Tests Missing Descriptions:** 0

**Numbering Mismatches:** 0

All numbered test numbers are unique.

This catalog lists all tests in the Rust codebase.

| Test # | Function Name | Description | File |
|--------|---------------|-------------|------|
| test0001 | `test0001_op_execution` | TEST0001: Run Op::perform and verify the returned value matches what the op was configured with | src/op.rs:44 |
| test0002 | `test0002_op_with_contexts` | TEST0002: Verify Op reads from DryContext and produces a formatted result using that data | src/op.rs:55 |
| test0003 | `test0003_op_default_rollback` | TEST0003: Confirm that the default rollback implementation is a no-op that always succeeds | src/op.rs:89 |
| test0004 | `test0004_op_custom_rollback` | TEST0004: Verify a custom rollback implementation is called and sets the rolled_back flag | src/op.rs:114 |
| test0005 | `test0005_perform_with_auto_logging` | TEST0005: Confirm the perform() utility wraps an op with automatic logging and returns its result | src/ops.rs:115 |
| test0006 | `test0006_caller_trigger_name` | TEST0006: Verify get_caller_trigger_name() returns a string containing the module path with "::" | src/ops.rs:128 |
| test0007 | `test0007_wrap_nested_op_exception` | TEST0007: Confirm wrap_nested_op_exception wraps an error with the op name in the message | src/ops.rs:136 |
| test0008 | `test0008_wrap_runtime_exception` | TEST0008: Verify wrap_runtime_exception converts a boxed std error into an OpError::ExecutionFailed | src/ops.rs:198 |
| test0009 | `test0009_dry_context_basic_operations` | TEST0009: Insert typed values into DryContext and verify get/contains work correctly | src/contexts.rs:257 |
| test0010 | `test0010_dry_context_builder` | TEST0010: Build a DryContext with chained with_value calls and verify all values are stored | src/contexts.rs:270 |
| test0011 | `test0011_wet_context_basic_operations` | TEST0011: Insert a reference into WetContext and retrieve it by type via get_ref | src/contexts.rs:281 |
| test0012 | `test0012_wet_context_builder` | TEST0012: Build a WetContext with chained with_ref calls and verify contains for each key | src/contexts.rs:299 |
| test0013 | `test0013_required_values` | TEST0013: Confirm get_required succeeds for present keys and returns an error for missing keys | src/contexts.rs:313 |
| test0014 | `test0014_context_merge` | TEST0014: Merge two DryContexts and verify values from both are accessible in the target | src/contexts.rs:322 |
| test0015 | `test0015_dry_context_type_mismatch_error` | TEST0015: Verify get_required returns a Type mismatch error when the stored type doesn't match | src/contexts.rs:333 |
| test0016 | `test0016_wet_context_type_mismatch_error` | TEST0016: Verify WetContext get_required returns a Type mismatch error when the stored ref type differs | src/contexts.rs:364 |
| test0017 | `test0017_control_flags` | TEST0017: Set and clear abort flags on DryContext and verify is_aborted and abort_reason reflect state | src/contexts.rs:395 |
| test0018 | `test0018_control_flags_merge` | TEST0018: Merge contexts with abort flags and confirm the target inherits the abort state correctly | src/contexts.rs:417 |
| test0019 | `test0019_get_or_insert_with` | TEST0019: Verify get_or_insert_with inserts when missing and returns existing without calling factory | src/contexts.rs:444 |
| test0020 | `test0020_get_or_compute_with` | TEST0020: Verify get_or_compute_with computes and stores a value using context data and skips recompute if present | src/contexts.rs:567 |
| test0021 | `test0021_metadata_builder` | TEST0021: Build OpMetadata with name, description, and schemas and verify all fields are populated | src/op_metadata.rs:265 |
| test0022 | `test0022_trigger_fuse` | TEST0022: Construct a TriggerFuse with data and verify the trigger name and dry context values | src/op_metadata.rs:288 |
| test0023 | `test0023_basic_validation` | TEST0023: Validate a DryContext against an input schema and confirm valid/invalid reports | src/op_metadata.rs:303 |
| test0024 | `test0024_simple_flat_outline` | TEST0024: Build a flat ListingOutline with depth-0 entries and verify max_depth, levels, and flatten count | src/structured_queries.rs:303 |
| test0025 | `test0025_hierarchical_outline` | TEST0025: Build a two-level outline with chapters and sections and verify depth, level counts, and flatten | src/structured_queries.rs:319 |
| test0026 | `test0026_complex_part_based_outline` | TEST0026: Build a three-level part/chapter/section outline and verify depth and per-level entry counts | src/structured_queries.rs:341 |
| test0027 | `test0027_flatten_preserves_hierarchy` | TEST0027: Flatten a nested outline and verify each entry's path reflects its ancestry correctly | src/structured_queries.rs:374 |
| test0028 | `test0028_schema_generation` | TEST0028: Call generate_outline_schema and verify the returned JSON contains all required definitions | src/structured_queries.rs:394 |
| test0029 | `test0029_logging_wrapper_success` | TEST0029: Wrap a successful op in LoggingWrapper and verify it passes through the result unchanged | src/wrappers/logging.rs:154 |
| test0030 | `test0030_logging_wrapper_failure` | TEST0030: Wrap a failing op in LoggingWrapper and verify the error includes the op name context | src/wrappers/logging.rs:184 |
| test0031 | `test0031_context_aware_logger` | TEST0031: Use create_context_aware_logger helper and verify the wrapped op returns its result | src/wrappers/logging.rs:219 |
| test0032 | `test0032_ansi_color_constants` | TEST0032: Verify ANSI color escape code constants have the expected ANSI sequence values | src/wrappers/logging.rs:234 |
| test0033 | `test0033_timeout_wrapper_success` | TEST0033: Wrap a fast op in TimeBoundWrapper and confirm it completes before the timeout | src/wrappers/timeout.rs:174 |
| test0034 | `test0034_timeout_wrapper_timeout` | TEST0034: Wrap a slow op in TimeBoundWrapper with a short timeout and verify a Timeout error is returned | src/wrappers/timeout.rs:203 |
| test0035 | `test0035_timeout_wrapper_with_name` | TEST0035: Create a named TimeBoundWrapper and verify the op succeeds and returns the expected value | src/wrappers/timeout.rs:236 |
| test0036 | `test0036_caller_name_wrapper` | TEST0036: Use create_timeout_wrapper_with_caller_name helper and verify the op result is returned | src/wrappers/timeout.rs:268 |
| test0037 | `test0037_logged_timeout_wrapper` | TEST0037: Use create_logged_timeout_wrapper to compose logging and timeout wrappers and verify success | src/wrappers/timeout.rs:296 |
| test0038 | `test0038_valid_input_output` | TEST0038: Run ValidatingWrapper with a valid input and verify the op executes and returns the result | src/wrappers/validating.rs:231 |
| test0039 | `test0039_invalid_input_missing_required` | TEST0039: Run ValidatingWrapper without a required input field and verify a Context validation error | src/wrappers/validating.rs:245 |
| test0040 | `test0040_invalid_input_out_of_range` | TEST0040: Run ValidatingWrapper with an input exceeding the schema maximum and verify a validation error | src/wrappers/validating.rs:263 |
| test0041 | `test0041_input_only_validation` | TEST0041: Use ValidatingWrapper::input_only and confirm input is validated while output is not | src/wrappers/validating.rs:281 |
| test0042 | `test0042_output_only_validation` | TEST0042: Use ValidatingWrapper::output_only and confirm output is validated while input is not | src/wrappers/validating.rs:316 |
| test0043 | `test0043_no_schema_validation` | TEST0043: Wrap an op with no schemas in ValidatingWrapper and confirm it still succeeds | src/wrappers/validating.rs:350 |
| test0044 | `test0044_metadata_transparency` | TEST0044: Verify ValidatingWrapper::metadata() delegates to the inner op's metadata unchanged | src/wrappers/validating.rs:377 |
| test0045 | `test0045_reference_validation` | TEST0045: Verify ValidatingWrapper checks reference_schema and rejects when required refs are missing | src/wrappers/validating.rs:389 |
| test0046 | `test0046_no_reference_schema` | TEST0046: Wrap an op with no reference schema in ValidatingWrapper and confirm it succeeds | src/wrappers/validating.rs:446 |
| test0047 | `test0047_batch_metadata_with_data_flow` | TEST0047: Build BatchMetadata from producer/consumer ops and verify only external inputs are required | src/batch_metadata.rs:246 |
| test0048 | `test0048_reference_schema_merging` | TEST0048: Build BatchMetadata from two ops with different reference schemas and verify union of required refs | src/batch_metadata.rs:268 |
| test0049 | `test0049_batch_op_success` | TEST0049: Run BatchOp with two succeeding ops and verify results contain both values in order | src/batch.rs:158 |
| test0050 | `test0050_batch_op_failure` | TEST0050: Run BatchOp where the second op fails and verify the batch returns an error | src/batch.rs:174 |
| test0051 | `test0051_batch_op_returns_all_results` | TEST0051: Run BatchOp with two ops and verify both result values are present in order | src/batch.rs:190 |
| test0052 | `test0052_batch_metadata_data_flow` | TEST0052: Verify BatchOp metadata correctly identifies only the externally-required input fields | src/batch.rs:208 |
| test0053 | `test0053_batch_reference_schema_merging` | TEST0053: Verify BatchOp merges reference schemas from all ops into a unified set of required refs | src/batch.rs:297 |
| test0054 | `test0054_batch_rollback_on_failure` | TEST0054: Run BatchOp where the third op fails and verify rollback is called on the first two but not the third | src/batch.rs:370 |
| test0055 | `test0055_batch_rollback_order` | TEST0055: Run BatchOp where the last op fails and verify rollback occurs in reverse (LIFO) order | src/batch.rs:451 |
| test0056 | `test0056_batch_rollback_on_failure_partial` | TEST0056: Run BatchOp where one op fails and verify rollback is triggered for succeeded ops | src/batch.rs:521 |
| test0057 | `test0057_abort_macro_without_reason` | TEST0057: Invoke the abort macro without a reason and verify the context is aborted with no reason string | src/control_flow_tests.rs:86 |
| test0058 | `test0058_abort_macro_with_reason` | TEST0058: Invoke the abort macro with a reason string and verify abort_reason matches | src/control_flow_tests.rs:106 |
| test0059 | `test0059_continue_loop_macro` | TEST0059: Use the continue_loop macro inside an op and verify the scoped continue flag is set in context | src/control_flow_tests.rs:126 |
| test0060 | `test0060_check_abort_macro` | TEST0060: Use check_abort macro to short-circuit when the abort flag is already set in context | src/control_flow_tests.rs:147 |
| test0061 | `test0061_batch_op_with_abort` | TEST0061: Run a BatchOp where the second op aborts and verify the batch stops and propagates the abort | src/control_flow_tests.rs:171 |
| test0062 | `test0062_batch_op_with_pre_existing_abort` | TEST0062: Start a BatchOp with an abort flag already set and verify it immediately returns Aborted | src/control_flow_tests.rs:194 |
| test0063 | `test0063_loop_op_with_continue` | TEST0063: Run a LoopOp where an op signals continue and verify subsequent ops in the iteration are skipped | src/control_flow_tests.rs:219 |
| test0064 | `test0064_loop_op_with_abort` | TEST0064: Run a LoopOp where an op aborts mid-loop and verify the loop terminates with the abort error | src/control_flow_tests.rs:244 |
| test0065 | `test0065_loop_op_with_pre_existing_abort` | TEST0065: Start a LoopOp with an abort flag already set and verify it immediately returns Aborted | src/control_flow_tests.rs:267 |
| test0066 | `test0066_complex_control_flow_scenario` | TEST0066: Nest a batch with a continue op inside a loop and verify results across all iterations | src/control_flow_tests.rs:291 |
| test0067 | `test0067_loop_op_basic` | TEST0067: Run a LoopOp for 3 iterations with 2 ops each and verify all 6 results in order | src/loop_op.rs:214 |
| test0068 | `test0068_loop_op_with_counter_access` | TEST0068: Run a LoopOp where each op reads the loop counter and verify values are 0, 1, 2 | src/loop_op.rs:233 |
| test0069 | `test0069_loop_op_existing_counter` | TEST0069: Start a LoopOp with a pre-initialized counter and verify it only executes the remaining iterations | src/loop_op.rs:250 |
| test0070 | `test0070_loop_op_zero_limit` | TEST0070: Run a LoopOp with a zero iteration limit and verify no ops are executed | src/loop_op.rs:268 |
| test0071 | `test0071_loop_op_builder_pattern` | TEST0071: Build a LoopOp with add_op chaining and verify all added ops run across all iterations | src/loop_op.rs:285 |
| test0072 | `test0072_loop_op_rollback_on_iteration_failure` | TEST0072: Run a LoopOp where the third op fails and verify succeeded ops are rolled back in reverse order | src/loop_op.rs:301 |
| test0073 | `test0073_loop_op_rollback_order_within_iteration` | TEST0073: Run a LoopOp where the last op fails and verify rollback occurs in LIFO order within the iteration | src/loop_op.rs:376 |
| test0074 | `test0074_loop_op_successful_iterations_not_rolled_back` | TEST0074: Run a LoopOp that fails on iteration 2 and verify previously completed iterations are not rolled back | src/loop_op.rs:446 |
| test0075 | `test0075_loop_op_mixed_iteration_with_rollback` | TEST0075: Run a LoopOp where op2 fails on iteration 1 and verify only op1 from that iteration is rolled back | src/loop_op.rs:514 |
| test0076 | `test0076_loop_op_continue_on_error` | TEST0076: Run a LoopOp configured to continue on error and verify subsequent iterations still execute | src/loop_op.rs:683 |
| test0077 | `test0077_dry_put_and_get` | TEST0077: Use dry_put! and dry_get! macros to store and retrieve a typed value by variable name | tests/macro_tests.rs:11 |
| test0078 | `test0078_dry_require` | TEST0078: Use dry_require! macro to retrieve a required value and verify error when key is missing | tests/macro_tests.rs:23 |
| test0079 | `test0079_dry_result` | TEST0079: Use dry_result! macro to store a final result and verify it is stored under both "result" and op name | tests/macro_tests.rs:40 |
| test0080 | `test0080_wet_put_ref_and_require_ref` | TEST0080: Use wet_put_ref! and wet_require_ref! macros to store and retrieve a service reference | tests/macro_tests.rs:60 |
| test0081 | `test0081_wet_put_arc` | TEST0081: Use wet_put_arc! to store an Arc-wrapped service and retrieve it via wet_require_ref! | tests/macro_tests.rs:73 |
| test0082 | `test0082_macros_in_op` | TEST0082: Run a full op that uses dry_require! and wet_require_ref! macros internally and verify the output | tests/macro_tests.rs:109 |
| test0083 | `test0083_error_handling_and_wrapper_chains` | TEST0083: Compose timeout and logging wrappers around a failing op and verify the error message includes the op name | tests/integration_tests.rs:39 |
| test0084 | `test0084_stack_trace_analysis` | TEST0084: Call get_caller_trigger_name from within a test and verify it reflects the integration test module path | tests/integration_tests.rs:65 |
| test0085 | `test0085_exception_wrapping_utilities` | TEST0085: Use wrap_nested_op_exception and verify the wrapped error contains both op name and original message | tests/integration_tests.rs:73 |
| test0086 | `test0086_timeout_wrapper_functionality` | TEST0086: Wrap a slow op in a short-timeout TimeBoundWrapper and verify the error is wrapped with logging context | tests/integration_tests.rs:104 |
| test0087 | `test0087_dry_and_wet_context_usage` | TEST0087: Run an op that retrieves a service from WetContext and reads config values from it | tests/integration_tests.rs:162 |
| test0088 | `test0088_batch_ops` | TEST0088: Run a BatchOp with two identical user-building ops and verify both produce the expected User struct | tests/integration_tests.rs:205 |
| test0089 | `test0089_wrapper_composition` | TEST0089: Compose TimeBoundWrapper and LoggingWrapper around a simple op and verify the result passes through | tests/integration_tests.rs:230 |
| test0090 | `test0090_perform_utility` | TEST0090: Use the perform() utility function directly and verify it returns the op result with auto-logging | tests/integration_tests.rs:259 |
| test0091 | `test0091_generate_thumbnail_macro` | TEST0091: Use the op! macro to generate a thumbnail op and verify it processes the file and updates context | test_macro_fix.rs:65 |
| test0092 | `test0092_process_file_macro` | TEST0092: Use the op! macro to generate a file processing op and verify context updates and result string | test_macro_fix.rs:85 |
| test0093 | `test0093_batch_len_and_is_empty` | TEST0093: Call BatchOp::len and is_empty on empty and non-empty batches | src/batch.rs:592 |
| test0094 | `test0094_batch_add_op` | TEST0094: Use add_op to dynamically add an op and verify it is executed | src/batch.rs:606 |
| test0095 | `test0095_batch_continue_on_error` | TEST0095: Run BatchOp::with_continue_on_error and verify it collects results past failures | src/batch.rs:620 |
| test0096 | `test0096_empty_batch_returns_empty` | TEST0096: Run an empty BatchOp and verify it returns an empty result vec | src/batch.rs:636 |
| test0097 | `test0097_nested_batch_rollback` | TEST0097: Verify nested BatchOp rollback propagates correctly when outer batch fails | src/batch.rs:646 |
| test0098 | `test0098_dry_context_merge_overwrites_keys` | TEST0098: Merge two DryContexts where keys overlap and verify the merging context's values win | src/contexts.rs:488 |
| test0099 | `test0099_wet_context_merge` | TEST0099: Merge two WetContexts and verify both sets of references are accessible in the target | src/contexts.rs:500 |
| test0100 | `test0100_dry_context_serde_roundtrip` | TEST0100: Serialize and deserialize a DryContext and verify all values survive the round-trip | src/contexts.rs:517 |
| test0101 | `test0101_dry_context_clone_is_independent` | TEST0101: Clone a DryContext and verify the clone is independent (mutations don't propagate) | src/contexts.rs:533 |
| test0102 | `test0102_dry_context_keys` | TEST0102: Verify DryContext::keys() returns all inserted keys | src/contexts.rs:543 |
| test0103 | `test0103_wet_context_keys` | TEST0103: Verify WetContext::keys() returns all inserted reference keys | src/contexts.rs:555 |
| test0104 | `test0104_op_error_display_execution_failed` | TEST0104: Verify OpError::ExecutionFailed displays with the correct message format | src/error.rs:126 |
| test0105 | `test0105_op_error_display_timeout` | TEST0105: Verify OpError::Timeout displays with the correct timeout_ms value | src/error.rs:133 |
| test0106 | `test0106_op_error_display_context` | TEST0106: Verify OpError::Context displays with the correct message format | src/error.rs:140 |
| test0107 | `test0107_op_error_display_aborted` | TEST0107: Verify OpError::Aborted displays with the correct message format | src/error.rs:147 |
| test0108 | `test0108_op_error_clone_execution_failed` | TEST0108: Clone an OpError::ExecutionFailed and verify the clone is identical | src/error.rs:154 |
| test0109 | `test0109_op_error_clone_timeout` | TEST0109: Clone OpError::Timeout and verify timeout_ms is preserved | src/error.rs:166 |
| test0110 | `test0110_op_error_clone_other_converts_to_execution_failed` | TEST0110: Clone OpError::Other and verify it becomes ExecutionFailed with the error message preserved | src/error.rs:177 |
| test0111 | `test0111_op_error_from_serde_json_error` | TEST0111: Convert a serde_json::Error into OpError via From impl | src/error.rs:250 |
| test0112 | `test0112_output_only_still_validates_references` | TEST0112: Verify ValidatingWrapper::output_only validates references even when input validation is disabled | src/wrappers/validating.rs:473 |
| test0113 | `test0113_loop_op_break_terminates_loop` | TEST0113: Run a LoopOp where an op sets the break flag and verify the loop terminates early | src/loop_op.rs:610 |
| test0114 | `test0114_loop_op_continue_on_error_skips_failed_iterations` | TEST0114: Run LoopOp::with_continue_on_error where an op fails and verify the loop continues | src/loop_op.rs:628 |
| test0115 | `test0115_loop_op_with_no_ops_produces_no_results` | TEST0115: Run an empty LoopOp with a non-zero limit and verify it produces no results | src/loop_op.rs:671 |
| test1730 | `test1730_wire_tokens_round_trip` | TEST1730: the wire vocabulary round-trips exactly and rejects unknowns. | src/failure.rs:87 |
| test1731 | `test1731_only_input_is_permanent` | TEST1731: only Input is permanent — the retry machinery keys on this. | src/failure.rs:102 |
| test1901 | `test1901_classified_accessors` | TEST1901: classified variants carry the emit source's identity through the accessors; unclassified variants are Internal with no code — the taxonomy's own rule (docs/failure-taxonomy.md). | src/error.rs:193 |
| test1902 | `test1902_clone_preserves_classification` | TEST1902: cloning a classified error preserves its full identity — the run-record path clones the terminal error before persisting. | src/error.rs:233 |
| test1903 | `test1903_wrap_preserves_classification` | TEST1903: wrapping preserves a classified failure's identity — the wrap enriches the human CHAIN only, never the class/code/reason (docs/failure-taxonomy.md). | src/ops.rs:153 |
---

*Generated from Rust source tree*
*Total tests: 120*
*Total numbered tests: 120*
*Total unnumbered tests: 0*
*Total numbered tests missing descriptions: 0*
*Total numbering mismatches: 0*
