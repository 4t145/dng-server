from ctypes.wintypes import SIZEL
import json
import sqlite3
import secrets
import sys
with open(sys.argv[1], 'r', encoding='utf-8') as jsonfile:
    lexobj = json.load(jsonfile)
    name = lexobj['name']
    lang = lexobj['lang']
    lexicon = lexobj['lexicon']
    size = len(lexicon)
    author = lexobj['author']
    version = lexobj['version']
    tags = ','.join(lexobj['tags'])
    brief = lexobj['brief']
    
    print('totally {} word(s)'.format(size))

    conn = sqlite3.connect('../sql/lexicon.db')
    print('db opened')
    cursor = conn.cursor()

    while True:
        lexcode = secrets.token_hex(4)
        cursor.execute('select * from sqlite_master where type = \'table\' and name = \':lexcode\';', {'lexcode': lexcode})
        if len(list(cursor)) != 0:
            continue
        else:
            with conn:
                print('lexcode is ' + lexcode)
                cursor.execute('insert into lexindex values (?, ?, ?, ?, ?, ?, ?, ?, ?);', (lexcode,name,lang,size,brief,author,version,tags,0))
                cursor.execute('create table \'{}\' (word text primary key);'.format(lexcode))
                for word in lexicon:
                    cursor.execute('insert into \'{}\' values (?)'.format(lexcode), (word,))

            conn.commit()
            print('finished')
            conn.close()
            break

