# Deploy no cluster (P7)

> Estado atual: validado **localmente via `docker-compose`** (server + Postgres). O deploy
> no home cluster (MetalLB-UDP + Postgres do cluster) está pronto nos manifests abaixo e
> entra quando houver acesso ao cluster/registry.

## Local (teste entre devs, sem cluster)
```bash
docker compose up --build        # sobe Postgres + servidor (UDP 5000)
# clientes desktop:
RAGNADOFUS_SERVER=127.0.0.1:5000 cargo run -p client
```

## Cluster (home k8s + MetalLB)
1. Build e push da imagem para um registry que o cluster acessa:
   ```bash
   docker build -t REGISTRY/ragnadofus-server:latest .
   docker push REGISTRY/ragnadofus-server:latest
   ```
   e ajuste `image:` em `deployment.yaml`.
2. Crie o Secret com a URL do Postgres do cluster (ns `postgres`):
   ```bash
   kubectl create secret generic ragnadofus-db \
     --from-literal=database-url='postgres://USER:PASS@postgres-postgresql.postgres.svc:5432/ragnadofus'
   ```
3. Aplique:
   ```bash
   kubectl apply -f k8s/deployment.yaml -f k8s/service-udp.yaml
   kubectl get svc ragnadofus-server   # pega o EXTERNAL-IP (LAN) atribuído pelo MetalLB
   ```
4. Clientes apontam para `EXTERNAL-IP:5000/udp` (`RAGNADOFUS_SERVER=<ip>:5000`).

> Kong é L7/HTTP e não serve UDP — por isso o `Service type=LoadBalancer protocol=UDP`
> dedicado (D4.2). Health/métricas HTTP (futuro) podem ir pelo Kong à parte.
