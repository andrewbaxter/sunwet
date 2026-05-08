create virtual table meta_fts using fts5(fulltext, content='meta', tokenize="trigram remove_diacritics 1");
insert into meta_fts(rowid, fulltext) select rowid, fulltext from meta;
create trigger meta_fts_insert after insert on meta begin
  insert into meta_fts(rowid, fulltext) values (new.rowid, new.fulltext);
end;
create trigger meta_fts_delete after delete on meta begin
  insert into meta_fts(meta_fts, rowid, fulltext) values('delete', old.rowid, old.fulltext);
end;
create trigger meta_fts_update after update on meta begin
  insert into meta_fts(meta_fts, rowid, fulltext) values('delete', old.rowid, old.fulltext);
  insert into meta_fts(rowid, fulltext) values (new.rowid, new.fulltext);
end;
