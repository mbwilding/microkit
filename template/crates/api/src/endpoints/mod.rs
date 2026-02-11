// Automatically discovers and registers all endpoint modules in src/endpoints/
// Just add a new .rs file and it will be auto-discovered!
microkit::discover_endpoints!("src/endpoints");
