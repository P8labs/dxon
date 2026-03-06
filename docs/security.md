# Security

## Trusted templates

dXon distinguishes between trusted and untrusted templates based on where they come from.

**Trusted sources:**

- Templates fetched from the official dXon registry (`https://github.com/P8labs/dxon-registry`)

**Untrusted sources:**

- Templates loaded from a URL other than the official registry
- Templates loaded from a local file path

## Trust warning

When you use a template from an untrusted source, dXon displays a warning before doing anything:

```
warning: this template is from an untrusted source
  source: https://example.com/templates/myenv.yaml

templates run commands inside your container during setup.
review the template before continuing.

do you want to proceed? [y/N]
```

You must confirm before dXon creates the container.

## Bypassing the prompt

If you have already reviewed the template and want to skip the confirmation, pass `--trust` (or `-y`):

```sh
dxon create myenv --template ./myenv.yaml --trust
```

This suppresses the prompt and proceeds directly. Use it in scripts or automation where interactive confirmation is not practical.

## Why this exists

Templates run arbitrary shell commands inside containers during setup. A malicious template could:

- Install unwanted software
- Exfiltrate files or credentials
- Modify your container in unexpected ways

The trust system is a reminder to review templates from unknown sources before running them. Official registry templates are reviewed before publication, so they skip the prompt.

## Best practices

- Prefer official registry templates when possible
- Review template YAML before using `--trust` on an untrusted source
- If you write templates for others to use, consider publishing them to the registry so they are reviewed and trusted by default
