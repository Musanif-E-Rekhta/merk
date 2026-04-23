package templates

#SurrealDBDeployment: {
	_config: #Config

	apiVersion: "apps/v1"
	kind:       "Deployment"
	metadata: {
		name:      "\(_config.metadata.name)-surrealdb"
		namespace: _config.metadata.namespace
		labels: {
			"app.kubernetes.io/name":       "\(_config.metadata.name)-surrealdb"
			"app.kubernetes.io/managed-by": "timoni"
		}
	}
	spec: {
		replicas: 1
		selector: matchLabels: "app.kubernetes.io/name": "\(_config.metadata.name)-surrealdb"
		template: {
			metadata: labels: {
				"app.kubernetes.io/name": "\(_config.metadata.name)-surrealdb"
			}
			spec: {
				containers: [{
					name:            "surrealdb"
					image:           "\(_config.surrealdb.image.repository):\(_config.surrealdb.image.tag)"
					imagePullPolicy: "IfNotPresent"
					// $(SURREALDB_PASS) is resolved by Kubernetes from the env entry below.
					args: [
						"start",
						"--log=info",
						"--user=\(_config.surrealdb.user)",
						"--pass=$(SURREALDB_PASS)",
						"memory",
					]
					ports: [{
						name:          "ws"
						containerPort: 8000
						protocol:      "TCP"
					}]
					env: [{
						name: "SURREALDB_PASS"
						valueFrom: secretKeyRef: {
							name: "\(_config.metadata.name)-secrets"
							key:  "SURREALDB_PASS"
						}
					}]
					resources: {
						requests: _config.surrealdb.resources.requests
						limits:   _config.surrealdb.resources.limits
					}
					livenessProbe: {
						httpGet: {
							path: "/health"
							port: 8000
						}
						initialDelaySeconds: 10
						periodSeconds:       30
						timeoutSeconds:      5
						failureThreshold:    3
					}
					readinessProbe: {
						httpGet: {
							path: "/health"
							port: 8000
						}
						initialDelaySeconds: 5
						periodSeconds:       10
						timeoutSeconds:      3
						failureThreshold:    3
					}
				}]
			}
		}
	}
}

#SurrealDBService: {
	_config: #Config

	apiVersion: "v1"
	kind:       "Service"
	metadata: {
		name:      "\(_config.metadata.name)-surrealdb"
		namespace: _config.metadata.namespace
		labels: {
			"app.kubernetes.io/name":       "\(_config.metadata.name)-surrealdb"
			"app.kubernetes.io/managed-by": "timoni"
		}
	}
	spec: {
		type: "ClusterIP"
		selector: "app.kubernetes.io/name": "\(_config.metadata.name)-surrealdb"
		ports: [{
			name:       "ws"
			port:       8000
			targetPort: "ws"
			protocol:   "TCP"
		}]
	}
}
