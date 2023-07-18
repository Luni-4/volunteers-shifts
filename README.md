# Goal

This repository contains the web app code for an Italian volunteering project
aimed at managing volunteers shifts. This web app can be employed for
other contexts too.

It is written in Rust using the [Rocket](https://rocket.rs/)
framework, while the front-end part has been written through the
[Bulma](https://bulma.io/) framework along with a pinch of JavaScript.

Deployment and hosting are performed by
[Shuttle-rs](https://www.shuttle.rs/) cloud-development platform.

The routes visible from the browser are in Italian language to ease volunteers
in remembering them. Also the labels and HTML objects descriptions are written
in Italian language.

## Visible Routes

Routes visible by **anyone** who accesses to them.

- `/` shows the page to authenticate in the volunteer space.
To login, it is necessary to insert the volunteer card identifier and surname.

- `/referenti` shows the page to authenticate in the referents space. This
page contains all sensible data about volunteers and their own shifts.
To login, it is necessary to insert a password.

- `/cookie` shows the cookie policy in Italian language. The policy is contained
in [cookie_policy.html.hbs](templates/cookie_policy.html.hbs), replace the
content of this file with your own language.

## Authenticated Routes

Routes accessible **only** through authentication.

- `/visualizzaturni` shows all shifts for a specific date. A
volunteer can move through different dates using select boxes which allows to
choose the week range and the respective day to visualize.
Once select boxes have been set up, volunteers names and surnames are
shown divided by task in the form of cards. One card per task.

- `/volontari` shows all volunteers data: identifier, name, surname and phone
number. These information is retrieved from an online CSV file and then
saved in the internal database when the web app is starting.
This page also contains, for each volunteer, a way to reach the `/turni` route.
Only a referent can access to this page.

- `gestoreturni?<card_id>` shows the page to insert and record shifts for the
volunteer associated to the card identifier, `card_id`, parameter.
More shifts can be added for a single week day, but with a different task.

- `/turni/<card_id>` shows all shifts associated to the `card_id` which
identifies a precise volunteer. Through this page, a volunteer can also remove
his/her own shifts.

- `download/database` downloads the whole web app database as a JSON file.

## Routes Redirections

The web app implements `PUT` and `DELETE` requests to change data. Only the
most notable ones are written below, the others can be visualized directly from
code. To process forms data, `POST` requests have been employed.

- `/updatevolunteers` updates volunteers information downloading the CSV
file with all their data.

- `/add/<card_id>/<shift_id>` adds a new shift, identified by the `shift_id`
query string, for the volunteer associated with the `card_id` query string.

- `/removeshift/<card_id>/<shift_id>` removes the shift, identified by the
`shift_id` query string, for the volunteer associated with the `card_id`
query string.

## Secrets Variables

Sensible data are passed to the web app through the `Secrets.toml` file.
In order to build the web app, you need to fill in the necessary variables
contained in the `Secrets.toml.example` file with your own data and then
rename the file to `Secrets.toml`.

## Cookies

Cookies are entirely managed by the `Rocket` framework.
Inside the web app, only technical cookies are present.
Cookies containing sensible data are encrypted, while those related to data flow
or graphical aspects can be inspected through browsers.

# With Shuttle

Instructions to build, deploy, and host the web app with Shuttle.

# Building

1. First off, download and install `Rust` on your computer following the
instructions provided [here](https://www.rust-lang.org/learn/get-started).

2. Install `Shuttle` as explained
[here](https://docs.shuttle.rs/introduction/installation).

3. Download this source code and enter its directory.

4. To run the web app on your pc:

    ```console
        cargo shuttle run
    ```

    To run the web app setting only the Italian timezone, you need to define an
    environment variable when running `cargo`.
    ```console
        CHRONO_TZ_TIMEZONE_FILTER="(Europe/Rome)" cargo shuttle run
    ```

    To run the web app on your network such that it could be accessible from
    your mobile phone:
    ```console
        cargo shuttle run --external [--port YOUR_PORT]
    ```

    Look at the [Smartphone Visualization](#smartphone-visualization) section
    in order to know how to visualize the web app on your mobile phone.

5. To deploy the code, you have to:

    5.1 Login on your `Shuttle` account with this command:
    ```console
        cargo shuttle login --api-key YOUR_API_TOKEN
    ```

    5.2 Create a new `Shuttle` project with the command:
    ```console
       cargo shuttle project start
    ```

    5.3 Deploy the code
    ```console
        cargo shuttle deploy
    ```

    5.4 Your deployed web app will be accessible at the following address:

    `https://{name}.shuttleapp.rs`

    where `{name}` is the web app name contained in the `Shuttle.toml` file.

# Without Shuttle

**This implementation is no more supported and it requires to change a little
part of the code to make it work!!**

Instructions to build and run the web app locally without Shuttle as cloud
platform.

## Building

1. First off, download and install Rust on your computer following the
instructions explained [here](https://www.rust-lang.org/learn/get-started).

2. Download this source code and enter its directory, then run:

```console
cargo run
```

3. To see your running web app, type in your browser the following address:
`127.0.0.1:8080`.

This is your `localhost` address with `8080` as port value.

## Smartphone building

If you want to access your web app from a smartphone connected to the same
network of the computer which runs the web app:

Run the web app on your computer changing the Rocket `profile` through the
environment variable `ROCKET_PROFILE`

```console
ROCKET_PROFILE=mobile cargo run
```

# Smartphone Visualization

To see your running web app, type in your smartphone browser
the following address `{address}:{port}`.
`{address}` is the IP address of your computer, while `{port}` is the relative
port value.

If you are building without Shuttle, you can find and change
this information in the `Rocket.toml` file under the voice `[mobile]`.

# Known Limitations

Because of this [issue](https://bugzilla.mozilla.org/show_bug.cgi?id=1510450),
HTML 5 form validation errors are not shown on `Firefox` mobile.

# License

Released under the [MIT License](LICENSES/MIT.txt).

# Acknowledgments

Thanks to [Bulma](https://bulma.io/), [Rocket](https://rocket.rs/), and
[Shuttle-rs](https://www.shuttle.rs/) frameworks, we can support volunteers
in helping others!
