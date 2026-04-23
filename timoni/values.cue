values: {
	image: {
		repository: "ghcr.io/musanif-e-rekhta/merk"
		tag:        "latest"
		digest:     ""
	}
	imagePullPolicy: "IfNotPresent"
	replicas:        1

	resources: {
		requests: {
			cpu:    "100m"
			memory: "128Mi"
		}
		limits: {
			cpu:    "500m"
			memory: "512Mi"
		}
	}

	service: {
		type: "ClusterIP"
		port: 9678
	}

	config: {
		host:      "0.0.0.0"
		enableTls: false
	}

	surrealdb: {
		image: {
			repository: "surrealdb/surrealdb"
			tag:        "latest"
		}
		user:      "root"
		namespace: "merk"
		database:  "merk"
		resources: {
			requests: {
				cpu:    "100m"
				memory: "256Mi"
			}
			limits: {
				cpu:    "500m"
				memory: "512Mi"
			}
		}
	}

	secrets: {
		jwtSecret:   "change-me-in-production-min-32-chars-required"
		surrealPass: "root"
	}
}
