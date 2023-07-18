# Goal

This repository contains the code of a web-app with the goal of managing the
volunteers shifts for some volunteering projects. This web-app can also be
adapted and modified for other contexts as well.

It is written in Rust using the [Rocket](https://rocket.rs/)
framework, while the front-end part has been written through the
[Bulma](https://bulma.io/) framework along with a pinch of JavaScript.

Deployment and hosting are performed by
[Shuttle-rs](https://www.shuttle.rs/) cloud-development platform.

## Visible Routes

Routes accessible without authentication.

- `/` shows the authentication page. To access the web-app and insert shifts,
it is necessary to insert a volunteer card identifier and surname.

- `/amministrazione` shows the administration page containing all sensible
data about volunteers and their shifts. It is necessary a password, known
by a small set of selected people, to access this area.

- `/visualizzaturni` shows all current and next week shifts as a table. A
volunteer can move through the days of a week using left and right arrows
and change weeks through buttons.

- `/cookie` shows the cookie policy in Italian language. The policy is contained
in [cookie_policy.html.hbs](templates/cookie_policy.html.hbs). Replace the
Italian content of this file with a different language if a cookie policy is
necessary.

## Authenticated Routes

Routes accessible **only** with authentication.

- `/turni` shows all shifts actually present within the database.
When this route is taken, every shift which is two weeks older than the
current date will be automatically deleted from the database. Through this page,
it is possible to remove volunteers shifts. Only an administrator can access
this page.

- `/volontari` shows all volunteers data retrieved from an online file and then
saved in the database. Each time this route is taken, the volunteer file will be
downloaded again in order to integrate in the web-app the most recent updates.
Only an administrator can access this page.

- `gestoreturni?{{card_id}}` shows the page to insert the volunteer shifts. More
shifts can be added for a single week day. Through this page volunteers can also
delete shifts which are already booked.

## Routes Redirections

- `/shift/{{id}}` removes the shift associated to the specified `id`.

- `/settimanacorrente/aggiungi/{{ index }}` adds a volunteer shift to the
current week using an `index` to identify the shift.

- `/settimanacorrente/rimuovi/{{ index }}` removes a volunteer shift to the
current week using an `index` to identify the shift.

- `/settimanasuccessiva/aggiungi/{{ index }}` adds a volunteer shift to the
next week using an `index` to identify the shift.

- `/settimanasuccessiva/rimuovi/{{ index }}` removes a volunteer shift to the
next week using an `index` to identify the shift.

## Secrets Variables

Sensible data are passed to the web-app through the `Secrets.toml` file.
In order to build the web app, you need to fill the necessary variables
contained in the `Secrets.toml.example` file with your own data and then
rename the file into `Secrets.toml`.

## Cookies

Cookies are entirely managed by the `Rocket` framework.
Inside the web-app, only technical cookies are present.
Cookies containing sensible data are encrypted, while those related to data flow
or graphical aspects can be inspected through browsers.

# With Shuttle

Instructions to build, deploy, and host the web app with Shuttle.

# Building

1. First off, download and install `Rust` on your computer following the
instructions explained [here](https://www.rust-lang.org/learn/get-started).

2. Install `Shuttle` as explained
[here](https://docs.shuttle.rs/introduction/installation).

3. Download this source code and enter its directory.

4. To run the web-app on your pc:

    ```console
        cargo shuttle run
    ```

    To run the web-app setting only the Italian timezone, you need to define an
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
[Shuttle-rs](https://www.shuttle.rs/) we can help people in organizing for 
helping other people!
