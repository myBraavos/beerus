use crate::exe::err::Error;

fn transform_function_invocation_json(
    json: &mut serde_json::Value,
) -> Result<(), Error> {
    if let Some(obj) = json.as_object_mut() {
        // Remove fields that gen doesn't expect
        obj.remove("steps");
        obj.remove("memory_holes");
        obj.remove("gas_consumed");

        // Transform execution_resources in nested function invocations
        if let Some(er) = obj.get_mut("execution_resources") {
            if let Some(er_obj) = er.as_object_mut() {
                // Remove fields gen doesn't expect
                er_obj.remove("steps");
                er_obj.remove("memory_holes");
                er_obj.remove("gas_consumed");
                er_obj.remove("da_gas_consumed");

                let mut computation_resources = serde_json::Map::new();
                if let Some(bic) = er_obj.remove("builtin_instance_counter") {
                    if let Some(bic_obj) = bic.as_object() {
                        if let Some(val) =
                            bic_obj.get("range_check_builtin_applications")
                        {
                            computation_resources.insert(
                                "range_check_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("pedersen_builtin_applications")
                        {
                            computation_resources.insert(
                                "pedersen_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("bitwise_builtin_applications")
                        {
                            computation_resources.insert(
                                "bitwise_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("ec_op_builtin_applications")
                        {
                            computation_resources.insert(
                                "ec_op_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("ecdsa_builtin_applications")
                        {
                            computation_resources.insert(
                                "ecdsa_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("keccak_builtin_applications")
                        {
                            computation_resources.insert(
                                "keccak_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("poseidon_builtin_applications")
                        {
                            computation_resources.insert(
                                "poseidon_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                    }
                }

                let mut new_er = serde_json::Map::new();
                for (k, v) in computation_resources {
                    new_er.insert(k, v);
                }
                *er = serde_json::Value::Object(new_er);
            }
        }

        // Recursively transform nested calls
        if let Some(calls) = obj.get_mut("calls") {
            if let Some(calls_arr) = calls.as_array_mut() {
                for call in calls_arr.iter_mut() {
                    transform_function_invocation_json(call)?;
                }
            }
        }
    }
    Ok(())
}

pub fn transform_trace_json(
    mut json: serde_json::Value,
    mut state_diff_json: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    // Transform execution_resources structure to match gen's format
    if let Some(obj) = json.as_object_mut() {
        // Remove fields that gen doesn't expect (like steps, memory_holes, gas_consumed at top level)
        obj.remove("steps");
        obj.remove("memory_holes");
        obj.remove("gas_consumed");

        // Transform execution_resources if present, or create it if missing
        let mut er = obj.remove("execution_resources");

        // If execution_resources is missing, try to extract it from execute_invocation
        if er.is_none() {
            if let Some(execute_inv) = obj.get("execute_invocation") {
                if let Some(ei_obj) = execute_inv.as_object() {
                    if let Some(ei_er) = ei_obj.get("execution_resources") {
                        er = Some(ei_er.clone());
                    }
                }
            }
        }

        if let Some(mut er_val) = er {
            if let Some(er_obj) = er_val.as_object_mut() {
                // Remove fields gen doesn't expect
                er_obj.remove("steps");
                er_obj.remove("memory_holes");
                er_obj.remove("gas_consumed");

                // Extract builtin_instance_counter and transform it
                let mut computation_resources = serde_json::Map::new();
                if let Some(bic) = er_obj.remove("builtin_instance_counter") {
                    if let Some(bic_obj) = bic.as_object() {
                        // Map apollo's field names to gen's field names
                        if let Some(val) =
                            bic_obj.get("range_check_builtin_applications")
                        {
                            computation_resources.insert(
                                "range_check_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("pedersen_builtin_applications")
                        {
                            computation_resources.insert(
                                "pedersen_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("bitwise_builtin_applications")
                        {
                            computation_resources.insert(
                                "bitwise_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("ec_op_builtin_applications")
                        {
                            computation_resources.insert(
                                "ec_op_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("ecdsa_builtin_applications")
                        {
                            computation_resources.insert(
                                "ecdsa_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("keccak_builtin_applications")
                        {
                            computation_resources.insert(
                                "keccak_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                        if let Some(val) =
                            bic_obj.get("poseidon_builtin_applications")
                        {
                            computation_resources.insert(
                                "poseidon_builtin_applications".to_string(),
                                val.clone(),
                            );
                        }
                    }
                }

                // Extract data_availability fields
                let mut data_availability = serde_json::Map::new();
                if let Some(da_gas) = er_obj.remove("da_gas_consumed") {
                    if let Some(da_obj) = da_gas.as_object() {
                        if let Some(val) = da_obj.get("l1_data_gas") {
                            data_availability
                                .insert("l1_data_gas".to_string(), val.clone());
                        }
                        if let Some(val) = da_obj.get("l1_gas") {
                            data_availability
                                .insert("l1_gas".to_string(), val.clone());
                        }
                        if let Some(val) = da_obj.get("l2_gas") {
                            data_availability
                                .insert("l2_gas".to_string(), val.clone());
                        }
                    }
                }

                // Reconstruct execution_resources in gen's format
                let mut new_er = serde_json::Map::new();
                // Flatten computation_resources into execution_resources
                for (k, v) in computation_resources {
                    new_er.insert(k, v);
                }
                if !data_availability.is_empty() {
                    new_er.insert(
                        "data_availability".to_string(),
                        serde_json::Value::Object(data_availability),
                    );
                }

                // Always insert execution_resources back (required by gen)
                obj.insert(
                    "execution_resources".to_string(),
                    serde_json::Value::Object(new_er),
                );
            } else {
                // If er_val is not an object, create an empty execution_resources
                obj.insert(
                    "execution_resources".to_string(),
                    serde_json::json!({}),
                );
            }
        } else {
            // If execution_resources is completely missing, create an empty one
            obj.insert(
                "execution_resources".to_string(),
                serde_json::json!({}),
            );
        }

        // Recursively transform nested structures (like in execute_invocation.calls)
        if let Some(execute_inv) = obj.get_mut("execute_invocation") {
            transform_function_invocation_json(execute_inv)?;
        }
        if let Some(validate_inv) = obj.get_mut("validate_invocation") {
            transform_function_invocation_json(validate_inv)?;
        }
        if let Some(fee_transfer_inv) = obj.get_mut("fee_transfer_invocation") {
            transform_function_invocation_json(fee_transfer_inv)?;
        }

        // Add missing fields in state_diff and insert it in traces
        if let Some(state_diff_obj) = state_diff_json.as_object_mut() {
            if let Some(storage_diffs_inv) =
                state_diff_obj.get_mut("storage_diffs")
            {
                // Transform storage_diffs from object format to array format
                if storage_diffs_inv.is_object() {
                    let mut storage_diffs_array = Vec::new();

                    // Iterate over each contract address
                    if let Some(storage_diffs_obj) =
                        storage_diffs_inv.as_object()
                    {
                        for (address, storage_entries) in
                            storage_diffs_obj.iter()
                        {
                            // Extract storage entries
                            let mut entries = Vec::new();
                            if let Some(entries_obj) =
                                storage_entries.as_object()
                            {
                                for (storage_key, value) in entries_obj.iter() {
                                    entries.push(serde_json::json!({
                                        "key": storage_key,
                                        "value": value
                                    }));
                                }
                            }

                            storage_diffs_array.push(serde_json::json!({
                                "address": address,
                                "storage_entries": entries
                            }));
                        }
                    }
                    *storage_diffs_inv =
                        serde_json::Value::Array(storage_diffs_array);
                }
            } else {
                state_diff_obj
                    .insert("storage_diffs".to_string(), serde_json::json!([]));
            }
            if let Some(storage_diffs_inv) =
                state_diff_obj.get_mut("storage_diffs")
            {
                // Transform storage_diffs from object format to array format
                if storage_diffs_inv.is_object() {
                    let mut storage_diffs_array = Vec::new();

                    // Iterate over each contract address
                    if let Some(storage_diffs_obj) =
                        storage_diffs_inv.as_object()
                    {
                        for (address, storage_entries) in
                            storage_diffs_obj.iter()
                        {
                            // Extract storage entries
                            let mut entries = Vec::new();
                            if let Some(entries_obj) =
                                storage_entries.as_object()
                            {
                                for (storage_key, value) in entries_obj.iter() {
                                    entries.push(serde_json::json!({
                                        "key": storage_key,
                                        "value": value
                                    }));
                                }
                            }

                            storage_diffs_array.push(serde_json::json!({
                                "address": address,
                                "storage_entries": entries
                            }));
                        }
                    }
                    *storage_diffs_inv =
                        serde_json::Value::Array(storage_diffs_array);
                }
            } else {
                state_diff_obj
                    .insert("storage_diffs".to_string(), serde_json::json!([]));
            }
            if let Some(nonces_inv) = state_diff_obj.get_mut("nonces") {
                // Transform nonces from object format to array format
                if nonces_inv.is_object() {
                    let mut nonces_array = Vec::new();

                    // Iterate over each contract address
                    if let Some(nonces_obj) = nonces_inv.as_object() {
                        for (address, nonce) in nonces_obj.iter() {
                            nonces_array.push(serde_json::json!({
                                "contract_address": address,
                                "nonce": nonce
                            }));
                        }
                    }
                    *nonces_inv = serde_json::Value::Array(nonces_array);
                }
            } else {
                state_diff_obj
                    .insert("nonces".to_string(), serde_json::json!([]));
            }
            if let Some(deployed_contracts_inv) =
                state_diff_obj.get_mut("deployed_contracts")
            {
                // Transform deployed_contracts from object format to array format
                if deployed_contracts_inv.is_object() {
                    let mut deployed_contracts_array = Vec::new();

                    // Iterate over each contract address
                    if let Some(deployed_contracts_obj) =
                        deployed_contracts_inv.as_object()
                    {
                        for (address, class_hash) in
                            deployed_contracts_obj.iter()
                        {
                            deployed_contracts_array.push(serde_json::json!({
                                "address": address,
                                "class_hash": class_hash
                            }));
                        }
                    }
                    *deployed_contracts_inv =
                        serde_json::Value::Array(deployed_contracts_array);
                }
            } else {
                state_diff_obj.insert(
                    "deployed_contracts".to_string(),
                    serde_json::json!([]),
                );
            }
            if state_diff_obj.get("deprecated_declared_classes").is_none() {
                state_diff_obj.insert(
                    "deprecated_declared_classes".to_string(),
                    serde_json::json!([]),
                );
            }
            if state_diff_obj.get("declared_classes").is_none() {
                state_diff_obj.insert(
                    "declared_classes".to_string(),
                    serde_json::json!([]),
                );
            }
            if state_diff_obj.get("replaced_classes").is_none() {
                state_diff_obj.insert(
                    "replaced_classes".to_string(),
                    serde_json::json!([]),
                );
            }
            if state_diff_obj.get("migrated_compiled_classes").is_none() {
                state_diff_obj.insert(
                    "migrated_compiled_classes".to_string(),
                    serde_json::json!([]),
                );
            }
            state_diff_obj.remove("class_hash_to_compiled_class_hash");
        }
        obj.insert("state_diff".to_string(), state_diff_json);
    }

    Ok(json)
}
