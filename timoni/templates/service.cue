package templates

#ServiceAccount: {
	_config: #Config

	apiVersion: "v1"
	kind:       "ServiceAccount"
	metadata: {
		name:      _config.metadata.name
		namespace: _config.metadata.namespace
		labels:    _config.metadata.labels
	}
}

#Service: {
	_config: #Config

	apiVersion: "v1"
	kind:       "Service"
	metadata: {
		name:      _config.metadata.name
		namespace: _config.metadata.namespace
		labels:    _config.metadata.labels
	}
	spec: {
		type: _config.service.type
		selector: "app.kubernetes.io/name": _config.metadata.name
		ports: [{
			name:       "http"
			port:       _config.service.port
			targetPort: "http"
			protocol:   "TCP"
		}]
	}
}
