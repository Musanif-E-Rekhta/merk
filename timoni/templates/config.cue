package templates

// #Config defines the schema and defaults for the instance values.
// metadata (name, namespace, labels) is injected by Timoni at apply time.
#Config: {
	metadata: {
		name:      string
		namespace: string
		labels: {[string]: string}
	}

	image: {
		repository: string
		tag:        string
		digest:     string | *""
	}

	imagePullPolicy: *"IfNotPresent" | "Always" | "Never"
	replicas:        *1 | int & >=1

	resources: {
		requests: {
			cpu:    string
			memory: string
		}
		limits: {
			cpu:    string
			memory: string
		}
	}

	service: {
		type: *"ClusterIP" | "NodePort" | "LoadBalancer"
		port: int & >0 & <=65535
	}

	config: {
		host:      string
		enableTls: bool | *false
	}

	surrealdb: {
		image: {
			repository: string
			tag:        string
		}
		user:      string
		namespace: string
		database:  string
		resources: {
			requests: {
				cpu:    string
				memory: string
			}
			limits: {
				cpu:    string
				memory: string
			}
		}
	}

	secrets: {
		jwtSecret:   string
		surrealPass: string
	}
}

// #Instance takes the config values and outputs all Kubernetes objects.
#Instance: {
	config: #Config

	objects: {
		sa:              #ServiceAccount & {_config:       config}
		cm:              #ConfigMap & {_config:            config}
		secret:          #Secret & {_config:               config}
		deploy:          #Deployment & {_config:           config}
		svc:             #Service & {_config:              config}
		surrealdbDeploy: #SurrealDBDeployment & {_config: config}
		surrealdbSvc:    #SurrealDBService & {_config:    config}
	}
}
