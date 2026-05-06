create virtual table subjobj_fts using fts5(fulltext, content='subjobj', tokenize="trigram remove_diacritics 1");
insert into subjobj_fts(rowid, fulltext) select rowid, fulltext from subjobj;
create trigger subjobj_fts_insert after insert on subjobj begin
  insert into subjobj_fts(rowid, fulltext) values (new.rowid, new.fulltext);
end;
create trigger subjobj_fts_delete after delete on subjobj begin
  insert into subjobj_fts(subjobj_fts, rowid, fulltext) values('delete', old.rowid, old.fulltext);
end;
create trigger subjobj_fts_update after update on subjobj begin
  insert into subjobj_fts(subjobj_fts, rowid, fulltext) values('delete', old.rowid, old.fulltext);
  insert into subjobj_fts(rowid, fulltext) values (new.rowid, new.fulltext);
end;
