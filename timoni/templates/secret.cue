package templates

#Secret: {
	_config: #Config

	apiVersion: "v1"
	kind:       "Secret"
	metadata: {
		name:      "\(_config.metadata.name)-secrets"
		namespace: _config.metadata.namespace
		labels:    _config.metadata.labels
	}
	// stringData lets Kubernetes handle base64 encoding automatically.
	type: "Opaque"
	stringData: {
		JWT_SECRET:     _config.secrets.jwtSecret
		SURREALDB_PASS: _config.secrets.surrealPass
	}
}
