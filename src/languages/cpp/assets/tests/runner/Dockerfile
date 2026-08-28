FROM gcc:15.2.1

WORKDIR /opt/ptip-ffi-tests

COPY runner/test.sh /usr/local/bin/ptip-ffi-tests-runner
RUN chmod +x /usr/local/bin/ptip-ffi-tests-runner

WORKDIR /assets/tests

ENTRYPOINT ["/usr/local/bin/ptip-ffi-tests-runner"]
