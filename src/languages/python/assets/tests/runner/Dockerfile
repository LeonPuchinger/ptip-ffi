FROM python:3.14-slim-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends make \
  && rm -rf /var/lib/apt/lists/*

ENV PYTHONPATH=/assets

WORKDIR /assets/tests

COPY runner/entrypoint.sh /usr/local/bin/ptip-ffi-python-tests-entrypoint
COPY runner/run-tests.sh /usr/local/bin/ptip-ffi-python-tests-runner
RUN chmod +x /usr/local/bin/ptip-ffi-python-tests-entrypoint /usr/local/bin/ptip-ffi-python-tests-runner

ENTRYPOINT ["/usr/local/bin/ptip-ffi-python-tests-entrypoint"]
CMD ["/usr/local/bin/ptip-ffi-python-tests-runner"]
