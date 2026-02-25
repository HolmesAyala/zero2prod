-- Add migration script here

INSERT INTO users (user_id, username, password_hash)
VALUES ('10ed350b-0349-4ad2-ac0d-34bf916e99e2',
        'admin',
        '$argon2id$v=19$m=15000,t=2,p=1$9P45eeqX9soYaXui2JpqzA$4EK2hjbgrT/7cuT+dZjxLNe42isHYmytbUcUBVw0Ro0');
