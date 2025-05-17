FROM envoyproxy/envoy:v1.22.0
COPY envoy.yaml /etc/envoy/envoy.yaml
RUN chmod 777 /etc/envoy/envoy.yaml
EXPOSE 8080
CMD ["/usr/local/bin/envoy", "-c", "/etc/envoy/envoy.yaml", "--service-cluster", "lambda-api-gateway", "--service-node", "lambda-api-gateway-1"]
