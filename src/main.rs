use std::error::Error;
use scraper::{Html, Selector};
use std::fs::File;
use std::io::prelude::*;
use futures::stream::StreamExt;

#[derive(Clone)]
struct Page {
    book_id: i32,
    id: i32,
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    const BOOK_ID: i32 = 624;

    let client = reqwest::Client::new();
    let book_max_page = get_book_max_page(&client, BOOK_ID).await?;

    println!("book_max_page: {}", book_max_page);

    let mut pages = Vec::<Page>::new();

    for i in 1..=book_max_page {
        let url = format!("https://www.haobook123.com/content/{}/{}", BOOK_ID, i);
        pages.push(Page{
            book_id: BOOK_ID,
            id: i,
            url
        });
    }

    let fetches = futures::stream::iter(
        pages.into_iter().map(download_page)
    ).buffer_unordered(8).collect::<Vec<Result<(), Box<dyn Error>>>>();

    fetches.await;

    merge_single_file(BOOK_ID).await?;

    println!("all done!");

    Ok(())
}

async fn get_book_max_page(client: &reqwest::Client, book_id: i32) -> Result<i32, Box<dyn Error>> {
    let resp = client.get(format!("https://www.haobook123.com/content/{}/{}", book_id, 1).as_str()).send().await?;
    let body = resp.text().await?;
    let doc = Html::parse_document(body.as_str());
    let mut count = 0;
    for ele in (&doc).select(&Selector::parse("#exampleFormControlSelect1").unwrap()) {
        count = ele.select(&Selector::parse("option").unwrap()).count();
    }
    return Ok(count as i32);
}

async fn download_page(page: Page) -> Result<(), Box<dyn Error>> {
    let url = page.url.as_str();
    let id = page.id;
    let resp = reqwest::get(url).await?;
    println!("page {} statusCode: {}", id, resp.status());

    let text = resp.text().await?;
    let doc = Html::parse_document(text.as_str());

    let mut content = String::new();
    for ele in (&doc).select(&Selector::parse("#pageContent").unwrap()) {
        for txt in ele.text() {
            content.push_str(String::from(txt).trim());
        }
    }

    std::fs::create_dir_all(format!("./data/{}/pages", page.book_id))?;
    let mut save_file = File::create(format!("./data/{}/pages/{}.txt", page.book_id, id))?;
    save_file.write_all(content.as_bytes())?;

    println!("page {} done!", id);

    Ok(())
}

async fn merge_single_file(book_id: i32) -> Result<(), Box<dyn Error>> {
    let mut output_file = File::create(format!("./data/{}/{}.txt", book_id, book_id))?;

    let dir = std::fs::read_dir(format!("./data/{}/pages", book_id))?;
    for entry in dir {
        let entry = entry?;
        let file_name = entry.file_name();
        let data = std::fs::read(format!("./data/{}/pages/{}", book_id, file_name.to_str().unwrap_or("invalid")))?;
        output_file.write(data.as_slice())?;
    }

    output_file.flush()?;

    Ok(())
}
