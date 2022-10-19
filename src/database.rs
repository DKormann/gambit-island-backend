

use postgrest::Postgrest;

const REST_URL:&str = "https://fdyzrdylphrevhqfghxd.supabase.co/rest/v1";

pub struct DBApi{
    pub secret : String,
}


pub fn get_api() -> DBApi{

    DBApi { secret: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImZkeXpyZHlscGhyZXZocWZnaHhkIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImlhdCI6MTY2NTE1ODQ4MCwiZXhwIjoxOTgwNzM0NDgwfQ.Qamroc1e1pjd_rinTMLF3jJCHPN_mQIISAXQWkwbi0c".into() }

}


pub async fn create_user (username: &str, email: &str, passhash: &str, secret: &String) -> Result<String,String>{

    {//check if username taken

        let client = Postgrest::new(REST_URL)
            .insert_header("apikey", &secret)
            .insert_header("Authorization", format!("Bearer {}",&secret));

        let check = client.from("users")
            .eq("username", "pater")
            .select("username")
            .execute()
            .await;
        
        println!("check result: {:?} ",check);

    }

    let client = Postgrest::new("https://fdyzrdylphrevhqfghxd.supabase.co/rest/v1")
        .insert_header("apikey", &secret)
        .insert_header("Authorization", format!("Bearer {}",&secret));

    let payload = format!(r#"[{{"username": "{}", "passwordhash": "{}", "email": "{}" }}]"#,username,passhash,email );

    let resp = client.from("users")
        .insert(payload)
        .execute()
        .await.or_else (|x|{Err(format!("cant execute query: {}",x))})?
        .text().await.or_else(|_|{Err("cant get string of result")})?;

    Ok(resp)
}

pub async fn check_user_credentials (username: &str, passhash: &str, secret: &String)-> Result<bool,String>{

    let client = Postgrest::new(REST_URL)
        .insert_header("apikey", &secret)
        .insert_header("Authorization", format!("Bearer {}", &secret));
    
    let resp = client.from("users")
        .eq("username",username)
        .eq("passwordhash", passhash)
        .select("username")
        .execute()
        .await.or_else(|x|{Err(format!("cant execute query {}",x))})?;
    
    // println!("got database response: {:?}",resp);

    let txt = resp.text().await.or_else(|e|{Err(format!("error getting text {:?}",e))})?;

    if txt == "[]".to_string(){
        Ok(false)
    }else{
        Ok(true)
    }
}

pub async fn get_player_score(username: &str, secret: &String)->Result<i32,String>{

    let client = Postgrest::new(REST_URL)
        .insert_header("apikey", &secret)
        .insert_header("Authorization", format!("Bearer {}",&secret));

    


    let score = client.from("pirate_gambit_scores")
    .eq("username", &username)
    .select("score")
    .execute().await.or_else(|e|{Err(format!("error getting score for id {} {:?}",&username,e))})?
    .text().await.or_else(|_|{Err(format!("cant get text"))})?;
    

    if score == "[]".to_string(){

        println!("inserting new score ");

        let id = get_id(&username, &secret).await?;

        let payload = format!(r#"[{{"id":"{}","username":"{}", "score":"{}"}}]"#,&id,username,100);

        client.from("pirate_gambit_scores")
        .insert(payload)
        .execute().await.or_else(|x|{Err(format!("cant execure query {:?}",x))})?
        .text().await.or_else(|_|{Err(format!("catnt get text"))})?;

        Ok(100)

    }else{

        let mut splits = score.split(":");
        splits.next();
        let score = splits.next().unwrap().split("}").next().unwrap();


        let res = score.parse::<i32>().or_else(|e|{Err(format!("cant parse score {} {:?}",score,e))})?;

        Ok(res)
    }

}

pub async fn get_id(username: &str, secret: &String)->Result<String,String>{
    let client = Postgrest::new(REST_URL)
        .insert_header("apikey", &secret)
        .insert_header("Authorization", format!("Bearer {}",&secret));

    let id = client.from("users")
    .eq("username" ,username)
    .select("id")
    .execute()
    .await.or_else(|x|{Err(format!("error getting user id {:?}",x))})?
    .text().await.or_else(|e|{Err(format!("error getting text {:?}",e))})?;


    //parsing id from answer like [{id: 32}
    let mut splits = id.split(":");
    splits.next();
    let id = splits.next().unwrap();
    splits = id.split("}");
    Ok(splits.next().unwrap().to_owned())

}

pub async fn set_player_score(username: &str, value: i32, secret: &String)->Result<(),String>{

    let client = Postgrest::new(REST_URL)
        .insert_header("apikey", &secret)
        .insert_header("Authorization", format!("Bearer {}",&secret));
    
    let id = get_id(username, secret).await?;
    
    let payload = format!(r#"[{{"score":"{}"}}]"#, value);

    client.from("pirate_gambit_scores")
    .eq("id",id)
    .update(payload)
    .execute().await.or_else(|_|{Err(format!("cant execute score update"))})?;
    Ok(())

}

