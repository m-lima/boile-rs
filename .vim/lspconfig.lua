return {
  rust_analyzer = {
    settings = {
      ['rust-analyzer'] = {
        cargo = {
          features = {
            'log',
            'log-headers',
            'log-spans',
            'log-multi-line',
            'log-tower',
            'rt',
            'rt-threads',
            'rt-shutdown',
            'panic',
            'server-h1',
            'server-h2',
          },
        },
      },
    },
  },
}
