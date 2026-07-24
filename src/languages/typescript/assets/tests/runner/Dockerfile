FROM node:22-bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends git openssh-client python3 make g++ \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/ptip-ffi-tests

COPY runner/package.json ./package.json
COPY runner/package-lock.json ./package-lock.json

RUN npm install --no-audit --no-fund

COPY runner/entrypoint.sh /usr/local/bin/ptip-ffi-tests-entrypoint
COPY runner/run-tests.sh /usr/local/bin/ptip-ffi-tests-runner
RUN chmod +x /usr/local/bin/ptip-ffi-tests-entrypoint /usr/local/bin/ptip-ffi-tests-runner

WORKDIR /assets/tests

ENTRYPOINT ["/usr/local/bin/ptip-ffi-tests-entrypoint"]
CMD ["/usr/local/bin/ptip-ffi-tests-runner"]