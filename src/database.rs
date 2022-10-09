

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

