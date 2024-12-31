create virtual table if not exists meta_fts using fts5(fulltext, content='meta');
create trigger if not exists meta_fts_insert after insert on meta begin
  insert into meta_fts(rowid, fulltext) values (new.rowid, new.fulltext);
end;
create trigger if not exists meta_fts_delete after delete on meta begin
  insert into meta_fts(meta_fts, rowid, fulltext) values('delete', old.rowid, old.fulltext);
end;
create trigger if not exists meta_fts_update after update on meta begin
  insert into meta_fts(meta_fts, rowid, fulltext) values('delete', old.rowid, old.fulltext);
  insert into meta_fts(rowid, fulltext) values (new.rowid, new.fulltext);
end;
