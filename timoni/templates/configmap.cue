package templates

#ConfigMap: {
	_config: #Config

	apiVersion: "v1"
	kind:       "ConfigMap"
	metadata: {
		name:      "\(_config.metadata.name)-config"
		namespace: _config.metadata.namespace
		labels:    _config.metadata.labels
	}
	data: {
		HOST:           _config.config.host
		PORT:           "\(_config.service.port)"
		ENABLE_TLS:     "\(_config.config.enableTls)"
		SURREALDB_URL:  "ws://\(_config.metadata.name)-surrealdb:8000"
		SURREALDB_USER: _config.surrealdb.user
		SURREALDB_NS:   _config.surrealdb.namespace
		SURREALDB_DB:   _config.surrealdb.database
	}
}
