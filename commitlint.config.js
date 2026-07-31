/** @type {import('@commitlint/types').UserConfig} */
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // Type must be one of the conventional commit types
    'type-enum': [
      2,
      'always',
      [
        'feat',
        'fix',
        'docs',
        'style',
        'refactor',
        'perf',
        'test',
        'build',
        'ci',
        'chore',
        'revert',
      ],
    ],
    // Subject line max length
    'header-max-length': [2, 'always', 100],
    // Subject must not end with a period
    'subject-full-stop': [2, 'never', '.'],
    // Subject must be in lower-case
    'subject-case': [2, 'always', 'lower-case'],
    // Body must have a blank line before it
    'body-leading-blank': [1, 'always'],
    // Footer must have a blank line before it
    'footer-leading-blank': [1, 'always'],
  },
};
