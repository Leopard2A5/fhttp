# FHTTP
File-based command line http client.

## What's this?
fhttp is not a curl replacement. It's meant to be a developer's tool to make http requests and store them as files, usually in a source code repository along with an application accepting http requests. It's inspired by tools like Postman, Insomnia and the IntelliJ http client.

## Installation
1. download the latest version [here](https://github.com/Leopard2A5/fhttp/releases)
1. rename the downloaded file?
1. make the file executable
1. make sure it's on your PATH

> Linux users: if you get
>
>`error while loading shared libraries: libssl.so.1.0.0: cannot open shared object file: No such file or directory`
>
>you need to install libssl1.0.0: `sudo apt-get install libssl1.0.0`

## Features
* Simply author a request in a *.http file
* Save a collection of requests right in your project repository
* Use profiles to easily switch between environments
* Resolve (environment) variables in your requests
* Add dependencies between requests
* Support for graphql requests

## Getting started
An http file consists of up to three parts:
* the method, url and headers
* body (optional)
* response handler (optional)

The method and url are the only mandatory parts. They follow the pattern of `<METHOD> <URL>`, e.g.  `GET www.google.com`.
You can specify headers by listing them underneath the first line:

```http request
GET http://google.com
Content-Type: application/json
# ignored: foobar
Api-key: 12345
```

To add a body, add an empty line after the headers part and write your body *without blank lines* in it:

```http request
POST http://localhost
Content-Type: application/json

{
  "foo": {
    "bar": 5
  }
}
```

## Response handlers
Suppose you've written a request file to get a JWT for authenticating yourself to another server:
```http request
POST http://authserver/authenticate
```

and this returns a json like this:
```json
{
  "access_token": "token",
  "expires": "2020-12-31T23:59:59Z",
  "roles": ["admin"]
}
```

If you're only interested in the access token you can add a response handler:
```
POST http://authserver/authenticate

> {%
  json $.access_token
%}
```

This will apply a json path expression to the response body and extract the `access_token` field.

With this, you can use your authentication request file in other request files:

```http request
GET http://protectedserver/resources
Authentication: Bearer ${request("authentication.http")}
```

When you now run `fhttp <file>`, fhttp will first run the authentication request, apply the response handler and then insert that value in place of the `${request("authentication.http")}` and run that.

## GraphQL requests
GraphQL requests are transmitted to the server as json, so naively a graphql request file would look like this:
```http request
POST http://graphqlserver
Content-Type: application/json

{
  "query": "query($var1: String!) { foo(var1: $var1) { field1 } }",
  "variables": {
    "var1": "val1"
  }
}
```

That's not very pretty, because it's a json payload and the query is transmitted as a string, we need to make it valid json. However, fhttp supports graphql requests directly. Just change the file's extension to *.gql.http or *.graphql.http and change it like this:
```
POST http://graphqlserver

query($var1: String!) {
  foo(var1: $var1) {
    field1
  }
}

{
  "var1": "val1"
}
``` 

Fhttp automatically sets the content-type to application/json, escapes the query string and constructs the json payload with the query and variables. Response handlers are also supported in graphql requests.

## Profiles
In the directory where you execute fhttp, you can create a file called `fhttp-config.json`, which allows you to create profiles to use in your requests. This file would typically look something like this:
```json
{
  "testing": {
    "variables": {
      "var1": "val1-testing"
    }
  },
  "production": {
    "variables": {
      "var1": "val1-production"
    }
  }
}
```

When you invoke fhttp with your requests you can call it with `-p <profile>` to use the corresponding variable definitions. These override existing environment variables.

### Pass secrets
If you use the popular password store [pass](https://www.passwordstore.org/), you can reference secrets from your profiles file. This allows you to keep secrets out of the profiles file and enables you to safely commit it.
```json
{
  "testing": {
    "variables": {
      "var1": {
        "path": "/path/inside/pass"
      }
    }
  }
}
```

fhttp will call the pass executable (must be in your PATH) to resolve the secret and insert it in your request wherever you've referenced the variable with `${env(variable)}`.
