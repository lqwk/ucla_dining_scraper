mod date;
mod model;
mod parse;
mod request;

use clap::{App, Arg, ArgMatches};
use model::storage::Storage;
use model::Menu;
use parse::parse_item;
use parse::parse_menu;
use request::menu_request;
use request::menu_request::MenuRequest;
use request::Downloadable;
use std::fs::OpenOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new("UCLA Menu Scraper")
        .version("1.0.0")
        .author("Qingwei Lan <qingweilandeveloper@gmail.com>")
        .about("Scrapes UClA dining website for menus and downloads the data")
        .arg(
            Arg::with_name("all")
                .short("a")
                .long("all")
                .help("Download all menus starting from current date")
                .conflicts_with("date"),
        )
        .arg(
            Arg::with_name("with-details")
                .short("d")
                .long("with-details")
                .help("Download menus along all item details"),
        )
        .arg(
            Arg::with_name("date")
                .long("date")
                .required_unless("all")
                .takes_value(true)
                .help("Specify the date (YYYY-MM-DD) for which menu to download"),
        )
        .arg(
            Arg::with_name("save")
                .long("save")
                .takes_value(true)
                .conflicts_with("save-pretty")
                .help("Save the downloaded data on disk in min JSON format"),
        )
        .arg(
            Arg::with_name("save-pretty")
                .long("save-pretty")
                .takes_value(true)
                .conflicts_with("save")
                .help("Save the downloaded data on disk in long pretty JSON format"),
        )
        .get_matches();

    run(&app).await
}

async fn run(app: &ArgMatches<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let requests = get_requests(app);

    for request in requests {
        print!(
            "Fetching {} {} for {} ... \t",
            request.date,
            request.meal.name(),
            request.restaurant.name()
        );

        if let Ok(body) = request.download().await {
            let mut menu = parse_menu::parse(body.as_str(), &request);
            println!("[done]");

            if app.is_present("with-details") {
                inflate_item_details(&mut menu).await?;
            }

            if app.is_present("save") || app.is_present("save-pretty") {
                // Get directory for which to save downloaded data
                let dir = {
                    if app.is_present("save") {
                        app.value_of("save").unwrap()
                    } else {
                        app.value_of("save-pretty").unwrap()
                    }
                };
                print!(
                    "Storing {} {} for {} on disk to {} ... \t",
                    request.date,
                    request.meal.name(),
                    request.restaurant.name(),
                    dir,
                );

                let filename = format!(
                    "{}-{}-{}{}",
                    request.date,
                    request.restaurant.url_name(),
                    request.meal.url_name(),
                    {
                        if app.is_present("save-pretty") {
                            "-pretty"
                        } else {
                            ""
                        }
                    }
                );
                let file = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(format!("{}/{}", dir, filename))?;

                if app.is_present("save") {
                    serde_json::to_writer(file, &menu.to_json_min())?;
                } else {
                    serde_json::to_writer_pretty(file, &menu.to_json())?;
                }

                println!("[done]");
            } else {
                println!("{}", menu);
            }
        }
    }

    Ok(())
}

async fn inflate_item_details(menu: &mut Menu) -> Result<(), Box<dyn std::error::Error>> {
    // Download all item details and inflate placeholders in Menu object
    for section in &mut menu.sections {
        for item in &mut section.items {
            item.set_details(parse_item::parse(
                item.details_request().download().await?.as_str(),
            ))
        }
    }
    Ok(())
}

fn get_requests(app: &ArgMatches) -> Vec<MenuRequest> {
    // Get all menu requests starting from today until a week later
    if app.is_present("all") {
        return menu_request::get_all_menu_requests();
    }

    // Get menu request for specific date
    let date = app.value_of("date").unwrap();
    return menu_request::menu_requests_for_dates(vec![date.into()]);
}
