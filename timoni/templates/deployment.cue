package templates

#Deployment: {
	_config: #Config

	apiVersion: "apps/v1"
	kind:       "Deployment"
	metadata: {
		name:      _config.metadata.name
		namespace: _config.metadata.namespace
		labels:    _config.metadata.labels
	}
	spec: {
		replicas: _config.replicas
		selector: matchLabels: "app.kubernetes.io/name": _config.metadata.name
		template: {
			metadata: labels: _config.metadata.labels
			spec: {
				serviceAccountName: _config.metadata.name
				securityContext: {
					runAsNonRoot: true
					runAsUser:    65534
					fsGroup:      65534
				}
				volumes: [{
					name: "tmp"
					emptyDir: {}
				}]
				containers: [{
					name:            _config.metadata.name
					image:           "\(_config.image.repository):\(_config.image.tag)"
					imagePullPolicy: _config.imagePullPolicy
					ports: [{
						name:          "http"
						containerPort: _config.service.port
						protocol:      "TCP"
					}]
					envFrom: [
						{configMapRef: name: "\(_config.metadata.name)-config"},
						{secretRef: name:    "\(_config.metadata.name)-secrets"},
					]
					volumeMounts: [{
						name:      "tmp"
						mountPath: "/tmp"
					}]
					resources: {
						requests: _config.resources.requests
						limits:   _config.resources.limits
					}
					livenessProbe: {
						httpGet: {
							path: "/api/v1/health"
							port: _config.service.port
						}
						initialDelaySeconds: 10
						periodSeconds:       30
						timeoutSeconds:      5
						failureThreshold:    3
					}
					readinessProbe: {
						httpGet: {
							path: "/api/v1/health"
							port: _config.service.port
						}
						initialDelaySeconds: 5
						periodSeconds:       10
						timeoutSeconds:      3
						failureThreshold:    3
					}
					securityContext: {
						allowPrivilegeEscalation: false
						readOnlyRootFilesystem:   true
						capabilities: drop: ["ALL"]
					}
				}]
			}
		}
	}
}
